// FX-05 lifecycle acceptance against a temporary real server and browser profile.
import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';

const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-fx05-'));
const config=path.join(scratch,'collector.toml');
const binary=path.join(root,'target/debug/collector-server');
const origin='http://localhost:20002';
const password=randomBytes(24).toString('hex');
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:20001"\nprivate_bind = "127.0.0.1:20002"\nlan_bind = "127.0.0.1:20003"\nallowed_origins = []\n`);
 const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',value=>logs+=value);server.stdout.on('data',value=>logs+=value);
 let ready=false;for(let attempt=0;attempt<1200;attempt++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,100));}assert.ok(ready,logs);
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

 // Long enough to keep playing while reimport and availability polling complete.
 const pcm=Buffer.alloc(44100*4*90);const wav=Buffer.alloc(44+pcm.length);wav.write('RIFF');wav.writeUInt32LE(wav.length-8,4);wav.write('WAVEfmt ',8);wav.writeUInt32LE(16,16);wav.writeUInt16LE(1,20);wav.writeUInt16LE(2,22);wav.writeUInt32LE(44100,24);wav.writeUInt32LE(44100*4,28);wav.writeUInt16LE(4,32);wav.writeUInt16LE(16,34);wav.write('data',36);wav.writeUInt32LE(pcm.length,40);
 const packageFiles=[{name:'original.wav',mimeType:'audio/wav',buffer:wav}];
 await editor.getByText('Choose individual files instead').click();await editor.getByLabel('Package files').setInputFiles(packageFiles);await editor.getByRole('button',{name:'Preview import'}).click();await editor.getByText('1 dump packages found.',{exact:false}).waitFor();await editor.getByLabel('Preserve originals without analysis or automatic lookup').check();await editor.getByRole('button',{name:'Preserve 1 package'}).click();
 const readItem=()=>page.evaluate(async id=>(await(await fetch('/api/v1/items/'+id)).json()),seeded.id);
 const waitItem=async predicate=>{for(let attempt=0;attempt<150;attempt++){const value=await readItem();if(predicate(value))return value;await page.waitForTimeout(200);}throw new Error('Item did not reach expected state: '+JSON.stringify(await readItem()));};
 await waitItem(item=>item.captures.length===1);
 await editor.getByText('Originals published. Metadata and optional preparation continue in this import and Work & history.',{exact:true}).waitFor();
 await editor.getByText('Upload complete.',{exact:false}).waitFor();
 const first=await readItem();assert.equal(first.captures.length,1);
 await editor.getByRole('button',{name:'Close curator'}).click();
 await page.getByRole('button',{name:'Add file to queue',exact:true}).click();
 const player=page.getByRole('region',{name:'Music player'});await player.getByRole('button',{name:'Play',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentTime>0.1;});
 await page.getByRole('button',{name:'Reimport cleanly',exact:true}).click();
 const clean=editor.getByRole('region',{name:'Clean reimport',exact:true});
 await clean.getByLabel('Analyze and look up metadata automatically').uncheck();
 assert.equal(await clean.getByLabel('Reset manual metadata overrides').isChecked(),false);assert.equal(await clean.getByLabel('Reset selected artwork').isChecked(),false);
 await clean.getByRole('button',{name:'Review clean reimport',exact:true}).click();await clean.getByRole('region',{name:'Review clean reimport',exact:true}).waitFor();
 await clean.getByRole('button',{name:'Start clean reimport',exact:true}).click();
 await waitItem(item=>item.captures.length===1&&item.captures[0].id!==first.captures[0].id);
 const second=await readItem();assert.equal(second.id,first.id);assert.equal(second.inventory_title,first.inventory_title);assert.equal(second.inventory_subtitle,first.inventory_subtitle);assert.equal(second.wishlist,true);assert.equal(second.domain.payload.artist,'Existing artist');
 // Cross the independent player's periodic access recheck after replacement.
 await page.waitForTimeout(5500);assert.equal(await player.locator('audio').evaluate(element=>element.paused),false);assert.equal(await player.locator('audio').getAttribute('data-representation-id'),first.captures[0].representations[0].id);
 await page.waitForFunction(()=>document.querySelector('[aria-label="Clean reimport"]')?.textContent.includes('succeeded'));await clean.getByRole('button',{name:'Plan another clean reimport'}).click();await clean.getByLabel('Package source').selectOption('upload');await clean.getByLabel('Replacement package files').setInputFiles(packageFiles);await clean.getByLabel('Analyze and look up metadata automatically').uncheck();await clean.getByRole('button',{name:'Review clean reimport',exact:true}).click();
 await page.route('**/api/v1/imports/*/files/*',route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'Interrupted replacement upload'}}}),{times:1});
 await clean.getByRole('button',{name:'Start clean reimport',exact:true}).click();await clean.getByRole('alert').filter({hasText:'Interrupted replacement upload'}).waitFor();
 assert.equal((await readItem()).captures[0].id,second.captures[0].id,'failed upload displaced usable capture');
 await editor.getByRole('button',{name:'Close curator'}).click();await page.getByRole('button',{name:'Reimport cleanly',exact:true}).click();await clean.getByLabel('Resume original package files').setInputFiles(packageFiles);await clean.getByRole('button',{name:'Resume upload',exact:true}).click();
 await waitItem(item=>item.captures.length===1&&item.captures[0].id!==second.captures[0].id);
 await editor.getByRole('button',{name:'Close curator'}).click();
 await page.setViewportSize({width:390,height:844});await page.getByRole('button',{name:'Reimport cleanly',exact:true}).click();await clean.getByRole('button',{name:'Plan another clean reimport'}).click();await clean.getByRole('button',{name:'Review clean reimport',exact:true}).click();assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);await page.screenshot({path:'/tmp/fx05-mobile-review.png',fullPage:true});
 await clean.getByRole('button',{name:'Back to options',exact:true}).click();await editor.getByRole('button',{name:'Close curator'}).click();
 page.once('dialog',dialog=>dialog.accept());await page.getByRole('button',{name:'Remove item',exact:true}).click();await page.getByRole('button',{name:'Restore entry',exact:true}).waitFor();await page.waitForFunction(()=>document.querySelector('.music-player audio')?.paused===true);await page.getByRole('button',{name:'Restore entry',exact:true}).click();await page.getByRole('button',{name:'Remove item',exact:true}).waitFor();await player.getByRole('button',{name:'Play',exact:true}).click();await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentTime>0.1;});
 assert.deepEqual(errors,[]);console.log('FX-05 browser passed: retained reuse, reviewed defaults, same identity, upload failure/reopen/resume, uninterrupted historical playback, mobile review, removal/restoration.');
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot());console.error(logs.slice(-3000));}throw error;}finally{if(browser)await browser.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(resolve=>server.once('exit',resolve)),new Promise(resolve=>setTimeout(()=>{server.kill('SIGKILL');resolve();},5000))]);}await rm(scratch,{recursive:true,force:true});}
