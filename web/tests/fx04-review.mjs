// FX-04 review regressions against a temporary real server and browser profile.
import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';

const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-fx04-review-'));
const config=path.join(scratch,'collector.toml');
const binary=path.join(root,'target/debug/collector-server');
const origin='http://localhost:19992';
const password=randomBytes(24).toString('hex');
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19991"\nprivate_bind = "127.0.0.1:19992"\nlan_bind = "127.0.0.1:19993"\nallowed_origins = []\n`);
 const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',value=>logs+=value);server.stdout.on('data',value=>logs+=value);
 let ready=false;for(let attempt=0;attempt<100;attempt++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 page=await browser.newPage();page.setDefaultTimeout(30000);const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection'}).click();const admin=page.getByRole('dialog');await admin.getByLabel('Collection name').fill('Review music');await admin.getByLabel('Collection type').selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection'}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const seeded=await page.evaluate(async()=>{
  const request=async(path,init={})=>{const response=await fetch('/api/v1'+path,{...init,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID(),...init.headers}});if(!response.ok)throw new Error(path+': '+await response.text());return response.json();};
  const collection=((await request('/collection-search?limit=100')).items).find(value=>value.name==='Review music');const id=crypto.randomUUID();
  const item=await request('/items/'+id,{method:'PUT',body:JSON.stringify({expected_revision:0,edit:{collection_id:collection.id,title:'Existing album',subtitle:'First edition',identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{artist:'Existing artist'}},identity:'unidentified',translations:[],identifiers:[],wishlist:true}})});
  return {id:item.id,collectionId:collection.id};
 });
 await page.getByRole('button',{name:'Add / import',exact:true}).click();const editor=page.getByRole('dialog');
 await editor.getByRole('button',{name:'Search existing entries'}).click();await editor.getByRole('searchbox',{name:'Search existing entries'}).fill('Existing album');await editor.locator('.destination-chooser .destination-results button').filter({hasText:'Existing album'}).click();
 const cue=Buffer.from('FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n');const pcm=Buffer.alloc(2352*10);
 await editor.getByText('Choose individual files instead').click();await editor.getByLabel('Files',{exact:true}).setInputFiles([{name:'disc.cue',mimeType:'text/plain',buffer:cue},{name:'disc.bin',mimeType:'application/octet-stream',buffer:pcm}]);await editor.getByRole('button',{name:'Preview import'}).click();await editor.getByText('1 dump packages found.',{exact:false}).waitFor();await editor.getByLabel('Save originals only').check();await editor.getByRole('button',{name:'Save 1 package'}).click();
 await page.waitForFunction(async id=>{const response=await fetch('/api/v1/items/'+id);return response.ok&&(await response.json()).captures.length===1;},seeded.id);
 await editor.getByRole('button',{name:'Details',exact:true}).click();
 assert.equal(await editor.getByLabel('Title',{exact:true}).inputValue(),'Existing album');
 assert.equal(await editor.getByLabel('Edition / variant details').inputValue(),'First edition');
 assert.equal(await editor.getByLabel('Artist').inputValue(),'Existing artist');
 assert.equal(await editor.getByLabel('Wishlist').isChecked(),true);
 await editor.getByLabel('Title',{exact:true}).fill('Draft after import');
 await page.evaluate(async id=>{const current=await(await fetch('/api/v1/items/'+id+'/metadata')).json();const response=await fetch('/api/v1/items/'+id+'/metadata',{method:'PUT',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({expected_revision:current.revision,change:{change:'set_field_override',field:'title',value:'Background metadata update',language:null}})});if(!response.ok)throw new Error(await response.text());},seeded.id);
 await page.waitForTimeout(3500);
 assert.equal(await editor.getByRole('button',{name:'Details',exact:true}).getAttribute('aria-current'),'page','background Item update changed the editing section');
 assert.equal(await editor.getByLabel('Title',{exact:true}).inputValue(),'Draft after import','background Item update discarded the title draft');
 await editor.getByRole('button',{name:'Save details'}).click();await editor.getByText('Entry saved.',{exact:false}).waitFor();
 const saved=await page.evaluate(async id=>(await(await fetch('/api/v1/items/'+id)).json()),seeded.id);
 assert.equal(saved.inventory_title,'Draft after import');assert.equal(saved.inventory_subtitle,'First edition');assert.equal(saved.wishlist,true);assert.equal(saved.domain.payload.artist,'Existing artist');assert.equal(saved.captures.length,1);
 // Synthetic provider responses exercise presentation without external provider requests.
 await page.route(`**/api/v1/items/${seeded.id}/metadata`,async route=>{
  if(route.request().method()!=='GET'){await route.continue();return;}
  const response=await route.fetch();const review=await response.json();
  const candidate=(title,strength,current,extra={})=>({identity:{provider:'org.junk.providers.musicbrainz',provider_record_id:title,provider_record_version:null},title:{value:title,language:'en'},subtitle:null,artwork:null,complete_identity:true,strength,provider_priority:0,current,selected:false,accepted_automatically:false,unmatched:false,details:null,music_layout:null,...extra});
  review.candidates=[candidate('Saved name choice','name',false,{selected:true,unmatched:true}),candidate('Accepted release','name',true,{accepted_automatically:true}),candidate('Detected disc match','disc_layout',true),candidate('Recorded release match','identifier',true),candidate('Obsolete name search','name',false),candidate('New name suggestion','name',true),candidate('Old disc match','disc_layout',false)];
  review.authority.selected_candidate=review.candidates[0].identity;
  await route.fulfill({response,json:review});
 });
 await editor.getByRole('button',{name:'Music',exact:true}).click();
 await editor.getByRole('button',{name:'Open Metadata workspace',exact:true}).click();
 const ledger=page.locator('.candidate-ledger');const titles=ledger.locator('.candidate > div:first-child > strong');await titles.getByText('Saved name choice',{exact:true}).waitFor();
 for(const title of ['Accepted release','Detected disc match','Recorded release match'])assert.equal(await titles.getByText(title,{exact:true}).count(),1);
 for(const title of ['Obsolete name search','New name suggestion','Old disc match'])assert.equal(await titles.getByText(title,{exact:true}).count(),0);
 const disclosure=ledger.getByRole('checkbox',{name:'Show text-search suggestions and earlier results'});
 await disclosure.check();await titles.getByText('Obsolete name search',{exact:true}).waitFor();
 assert.equal(await titles.getByText('New name suggestion',{exact:true}).count(),1);
 await disclosure.uncheck();assert.equal(await titles.getByText('Obsolete name search',{exact:true}).count(),0);
 assert.deepEqual(errors,[]);console.log('FX-04 review browser passed: existing destination drafts, preserved fields, stable section during background updates, and album-match/history disclosure.');
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot());console.error(logs.slice(-3000));}throw error;}finally{if(browser)await browser.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(resolve=>server.once('exit',resolve)),new Promise(resolve=>setTimeout(()=>{server.kill('SIGKILL');resolve();},5000))]);}await rm(scratch,{recursive:true,force:true});}
