// FX-04 scale and navigation regression against a temporary real server/profile.
import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';

const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-fx04-'));
const config=path.join(scratch,'collector.toml');
const binary=path.join(root,'target/debug/collector-server');
const password=randomBytes(24).toString('hex');
const origin='http://localhost:19982';
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19981"\nprivate_bind = "127.0.0.1:19982"\nlan_bind = "127.0.0.1:19983"\nallowed_origins = []\n`);
 const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',chunk=>logs+=chunk);server.stdout.on('data',chunk=>logs+=chunk);
 let ready=false;for(let n=0;n<100;n++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 page=await browser.newPage({viewport:{width:1440,height:1000}});page.setDefaultTimeout(30000);const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection'}).click();const admin=page.getByRole('dialog');await admin.getByLabel('Collection name').fill('Scale music');await admin.getByLabel('Collection type').selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection'}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const seeded=await page.evaluate(async()=>{
  const request=async(path,init={})=>{const response=await fetch('/api/v1'+path,{...init,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID(),...init.headers}});if(!response.ok)throw new Error(path+': '+await response.text());return response.json();};
  const collection=((await request('/collection-search?limit=100')).items).find(value=>value.name==='Scale music');
  const ids=new Array(3000),imports=new Array(3000);let next=0;
  await Promise.all(Array.from({length:1},async()=>{for(;;){const index=next++;if(index>=3000)return;const id=crypto.randomUUID();const response=await request('/items/'+id,{method:'PUT',body:JSON.stringify({expected_revision:0,edit:{collection_id:collection.id,title:index===2998?'Album 2999':'Album '+String(index).padStart(4,'0'),subtitle:index===2998?'Edition alternate':'Edition '+Math.floor(index/1000),identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{artist:index===2998?'Artist alternate':'Artist '+Math.floor(index/1000)}},identity:'unidentified',translations:[],identifiers:[],wishlist:false}})});ids[index]=response.id;}}));
  next=0;await Promise.all(Array.from({length:1},async()=>{for(;;){const index=next++;if(index>=3000)return;const result=await request('/imports',{method:'POST',body:JSON.stringify({collection_id:collection.id,package_name:'Package '+String(index).padStart(4,'0'),files:[{name:'disc.bin',bytes:4,sha256:null}]})});imports[index]=result.id;}}));
  return {collection:collection.id,ids,imports};
 });
 await page.getByRole('button',{name:'Work & history',exact:true}).click();const work=page.getByRole('dialog',{name:'Work & history'});await work.locator('.job-list > article').first().waitFor();assert.equal(await work.locator('.job-list > article').count(),25);
 await work.getByLabel('Search work').fill('Package 2999');await work.getByText('Package 2999',{exact:true}).waitFor();const row=work.locator('.job-list > article').filter({hasText:'Package 2999'});assert.equal(await row.count(),1);
 let catalogReads=0;const read=request=>{const url=new URL(request.url());if(url.pathname==='/api/v1/items')catalogReads++;};page.on('request',read);
 await row.getByRole('button',{name:'Open work'}).click();await work.getByRole('region',{name:'Selected work'}).waitFor();assert.equal(await work.locator('.job-list').count(),0);assert.ok(page.url().includes(`work=import%3A${seeded.imports[2999]}`));assert.equal(catalogReads,0,'opening import details fetched the catalog');
 await work.getByRole('button',{name:'Back to history'}).click();await row.waitFor();assert.equal(await work.getByLabel('Search work').inputValue(),'Package 2999');
 await row.getByRole('button',{name:'Open work'}).click();await page.goBack();await row.waitFor();await page.goForward();await page.getByRole('region',{name:'Selected work'}).waitFor();await page.reload();await page.getByRole('region',{name:'Selected work'}).waitFor();assert.equal(await page.locator('.job-list').count(),0);await page.getByRole('button',{name:'Close work history'}).click();
 await page.getByRole('button',{name:'Add / import'}).click();const editor=page.getByRole('dialog',{name:'Add or import'});await editor.getByRole('button',{name:'Search existing entries'}).click();await editor.getByRole('searchbox',{name:'Search existing entries'}).fill('Album');await editor.locator('.destination-chooser .destination-results button').first().waitFor();assert.equal(await editor.locator('.destination-chooser .destination-results button').count(),50);await editor.getByRole('button',{name:'More results'}).click();await editor.locator('.destination-chooser .destination-results button').nth(99).waitFor();await editor.getByRole('searchbox',{name:'Search existing entries'}).fill('Album 2999');await editor.locator('.destination-chooser .destination-results button').first().waitFor();assert.equal(await editor.locator('.destination-chooser .destination-results button').count(),2);const destination=editor.locator('.destination-chooser .destination-results button').filter({hasText:'Artist 2'});await destination.waitFor();assert.match(await destination.innerText(),/Edition 2/);await destination.click();assert.match(await editor.locator('.destination-chooser').innerText(),/Album 2999/);
 const cue=Buffer.from('FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n');const pcm=Buffer.alloc(2352*10);
 await editor.getByText('Choose individual files instead').click();await editor.getByLabel('Package files').setInputFiles([{name:'disc.cue',mimeType:'text/plain',buffer:cue},{name:'disc.bin',mimeType:'application/octet-stream',buffer:pcm}]);await editor.getByRole('button',{name:'Preview import'}).click();await editor.getByText('1 dump packages found.',{exact:false}).waitFor();
 await editor.getByRole('button',{name:'Enter inventory manually'}).click();await editor.getByRole('button',{name:'Import dumps'}).click();await editor.getByText('1 dump packages found.',{exact:false}).waitFor();
 await editor.getByRole('button',{name:'Preserve 1 package'}).click();await page.waitForFunction(async id=>{const response=await fetch('/api/v1/items/'+id);return response.ok&&(await response.json()).captures.length>0;},seeded.ids[2999],{timeout:30000});
 await page.waitForFunction(()=>document.querySelector('[role=dialog]')?.getAttribute('data-unsaved')==='false');await editor.getByRole('button',{name:'Close curator'}).click();await editor.waitFor({state:'detached'});await page.setViewportSize({width:390,height:844});await page.getByRole('button',{name:'Work & history',exact:true}).click();await page.getByRole('dialog').getByLabel('Search work').fill('Package 2999');await page.getByRole('dialog').locator('.job-list > article').first().getByRole('button',{name:'Open work'}).click();assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.keyboard.press('Tab');assert.ok(await page.getByRole('dialog').evaluate(element=>element.contains(document.activeElement)));
 assert.deepEqual(errors,[]);console.log('FX-04 browser scale passed: 3000 entries, 3000 imports, paged destination, attachment, separate direct-linked history, draft and mobile.');
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot());console.error(logs.slice(-5000));await page.screenshot({path:'/tmp/fx04-browser-failure.png',fullPage:true});}throw error;}finally{if(browser)await browser.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(resolve=>server.once('exit',resolve)),new Promise(resolve=>setTimeout(()=>{server.kill('SIGKILL');resolve();},5000))]);}await rm(scratch,{recursive:true,force:true});}
