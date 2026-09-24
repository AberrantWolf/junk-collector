// FX-03 real-server history lifecycle regression. All archive/state roots are temporary.
import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-work-'));
const config=path.join(scratch,'collector.toml');
const binary=path.join(root,'target/debug/collector-server');
const password=randomBytes(24).toString('hex');
const origin='http://localhost:19962';
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19961"\nprivate_bind = "127.0.0.1:19962"\nlan_bind = "127.0.0.1:19963"\nallowed_origins = []\n`);
 const init=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(init.status,0,init.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',v=>logs+=v);server.stdout.on('data',v=>logs+=v);
 let ready=false;for(let n=0;n<100;n++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const context=await browser.newContext({viewport:{width:1440,height:1000}});page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(15000);
 await page.goto(origin);for(const name of ['app.js','app.css','work-history.js'])assert.equal(await page.evaluate(async name=>await(await fetch('/'+name)).text(),name),await readFile(path.join(root,'web/dist',name),'utf8'),'server contains stale '+name);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 const admin=page.getByRole('dialog');await admin.getByLabel('Collection name').fill('Work games');await admin.getByLabel('Collection type').selectOption('org.junk.games');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await page.getByRole('button',{name:'Close administration'}).click();
 const fixture=await page.evaluate(async()=>{
  const request=async(path,init={})=>{const response=await fetch('/api/v1'+path,{...init,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID(),...init.headers}});if(!response.ok)throw new Error(path+': '+await response.text());return response.status===204?null:response.json();};
  const collection=(await request('/collections')).find(c=>c.name==='Work games');
  const imports=[];for(let i=0;i<28;i++)imports.push(await request('/imports',{method:'POST',body:JSON.stringify({collection_id:collection.id,package_name:`Package ${String(i).padStart(2,'0')}`,files:[{name:'disc.bin',bytes:4,sha256:null}]})}));
  let staged=await request(`/imports/${imports[0].id}/files/0?offset=0`,{method:'PUT',headers:{'content-type':'application/octet-stream'},body:new Uint8Array([1,2,3,4])});
  staged=await request(`/imports/${staged.id}/review`,{method:'PUT',body:JSON.stringify({expected_revision:staged.revision,identity:{identity:'unidentified'},association:{item_id:null,specimen_id:null,component_id:null,carrier_id:null},expected_item_revision:null})});
  const accepted=await request(`/imports/${staged.id}/publish`,{method:'POST'});for(let n=0;n<100;n++){const j=await request('/jobs/'+accepted.job_id);if(j.state==='completed')break;if(j.state==='failed')throw new Error(j.error);await new Promise(r=>setTimeout(r,100));}
  await request(`/imports/${imports[1].id}/files/0?offset=0`,{method:'PUT',headers:{'content-type':'application/octet-stream'},body:new Uint8Array([1,2])});
  return {collection:collection.id,imports:imports.map(i=>i.id),capture:staged.capture_id};
 });
 await page.getByRole('button',{name:'Work & history',exact:true}).click();const work=page.getByRole('dialog',{name:'Work & history'});
 const rows=work.locator('.job-list > article');await rows.nth(24).waitFor();assert.equal(await rows.count(),25);await work.getByRole('button',{name:'Load older work'}).click();await rows.nth(27).waitFor();assert.equal(await rows.count(),28);await work.getByRole('button',{name:'Refresh work',exact:true}).click();await page.waitForTimeout(700);assert.equal(await rows.count(),28,'poll discarded older page');
 await work.getByLabel('Search work',{exact:true}).fill('Package 0');await page.waitForFunction(()=>document.querySelectorAll('.work-history .job-list > article').length===10);
 await work.getByLabel('Select Package 00',{exact:true}).check();await work.getByLabel('Select Package 01',{exact:true}).check();await work.getByRole('button',{name:'Dismiss history',exact:true}).click();
 const review=work.getByRole('region',{name:'Review selected work changes'});assert.match(await review.innerText(),/Not eligible; will be skipped/);await review.getByRole('button',{name:'Apply to eligible selected work'}).click();await review.getByRole('status').waitFor();assert.match(await review.innerText(),/Package 01: not eligible; unchanged/);
 await review.getByRole('button',{name:'Close review'}).click();await work.getByLabel('Show dismissed history').check();await work.getByLabel('Select Package 00',{exact:true}).check();await work.getByRole('button',{name:'Restore history',exact:true}).click();await review.getByRole('button',{name:'Apply to eligible selected work'}).click();await review.getByRole('status').waitFor();await review.getByRole('button',{name:'Close review'}).click();await work.getByLabel('Show dismissed history').uncheck();
 await work.getByLabel('Select Package 01',{exact:true}).check();await work.getByRole('button',{name:'Cancel and clear unfinished work',exact:true}).click();assert.match(await review.innerText(),/2 B staging/);await review.getByRole('button',{name:'Apply to eligible selected work'}).click();await review.getByRole('status').waitFor();await review.getByRole('button',{name:'Close review'}).click();
 await work.getByLabel('Show dismissed history').check();await rows.filter({hasText:'Package 01'}).getByText('Reclaimed 2 B · pending 0 B',{exact:true}).waitFor({timeout:30000});
 await work.getByRole('button',{name:'Close work history'}).click();await page.reload();await page.getByRole('button',{name:'Work & history',exact:true}).click();await work.getByLabel('Show dismissed history').check();await rows.filter({hasText:'Package 01'}).waitFor();
 await page.setViewportSize({width:390,height:844});await page.screenshot({path:'/tmp/fx03-work-mobile.png',fullPage:true});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.keyboard.press('Tab');assert.ok(await work.evaluate(el=>el.contains(document.activeElement)),'focus escaped dialog');
 await work.getByLabel('Show dismissed history').uncheck();await work.getByLabel('Search work',{exact:true}).fill('Package 02');await rows.first().getByRole('button',{name:'Open work',exact:true}).click();await work.getByRole('region',{name:'Selected work'}).getByLabel('Original package files',{exact:true}).setInputFiles({name:'disc.bin',mimeType:'application/octet-stream',buffer:Buffer.from([1,2,3,4])});await work.getByRole('button',{name:'Resume upload',exact:true}).click();await work.getByText('analyzing · 1 file',{exact:true}).waitFor();
 await page.route('**/api/v1/work?*',r=>r.fulfill({status:503,json:{error:{code:'temporarily_unavailable',message:'History temporarily unavailable'}}}),{times:1});await work.getByRole('button',{name:'Refresh work',exact:true}).click();await work.getByRole('alert').waitFor();await work.getByRole('button',{name:'Refresh work',exact:true}).click();await work.getByRole('alert').waitFor({state:'detached'});
 assert.deepEqual(errors,[]);console.log(`FX-03 browser passed (${await browser.version()}): 28 imports, stable loaded pages, mixed eligibility, dismiss/restore, measured cleanup, reload, manual resume, mobile, keyboard and status recovery.`);
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot());await page.screenshot({path:'/tmp/fx03-browser-failure.png',fullPage:true});}throw error;}finally{
 if(browser)await browser.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(r=>server.once('exit',r)),new Promise(r=>setTimeout(()=>{server.kill('SIGKILL');r();},5000))]);}await rm(scratch,{recursive:true,force:true});
}
