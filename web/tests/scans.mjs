// FX-02 real-server scan lifecycle regression. All archive/state roots are temporary.
import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-scans-'));
const config=path.join(scratch,'collector.toml');
const binary=path.join(root,'target/debug/collector-server');
const password=randomBytes(24).toString('hex');
const origin='http://localhost:19952';
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19951"\nprivate_bind = "127.0.0.1:19952"\nlan_bind = "127.0.0.1:19953"\nallowed_origins = []\n`);
 const init=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(init.status,0,init.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',v=>logs+=v);server.stdout.on('data',v=>logs+=v);
 let ready=false;for(let n=0;n<100;n++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const context=await browser.newContext({viewport:{width:1440,height:1000}});page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(15000);
 await page.goto(origin);for(const name of ['app.js','app.css','curation.js'])assert.equal(await page.evaluate(async name=>await(await fetch('/'+name)).text(),name),await readFile(path.join(root,'web/dist',name),'utf8'),'server contains stale '+name);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 const admin=page.getByRole('dialog');await admin.getByLabel('Collection name').fill('Scan games');await admin.getByLabel('Collection type').selectOption('org.junk.games');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await page.getByRole('button',{name:'Close administration'}).click();
 const fixture=await page.evaluate(async()=>{
  const request=async(path,init={})=>{const response=await fetch('/api/v1'+path,{...init,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID(),...init.headers}});if(!response.ok)throw new Error(path+': '+await response.text());return response.json();};
  const collection=(await request('/collections')).find(c=>c.name==='Scan games');const id=crypto.randomUUID();
  const item=await request('/items/'+id,{method:'PUT',body:JSON.stringify({expected_revision:0,edit:{collection_id:collection.id,title:'Slow real scan',subtitle:'',identity_link:{kind:'none'},domain_edit:{module:'org.junk.games',fields:{platform:'nes'}},identity:'unidentified',translations:[],identifiers:[],wishlist:false}})});
  let staged=await request('/imports',{method:'POST',body:JSON.stringify({collection_id:collection.id,package_name:'scan package',files:[{name:'game.bin',bytes:64*1024*1024,sha256:null}]})});
  const chunk=new Uint8Array(2*1024*1024);for(let offset=0;offset<64*1024*1024;offset+=chunk.length)staged=await request(`/imports/${staged.id}/files/0?offset=${offset}`,{method:'PUT',headers:{'content-type':'application/octet-stream'},body:chunk});
  const reviewed=await request(`/imports/${staged.id}/review`,{method:'PUT',body:JSON.stringify({expected_revision:staged.revision,identity:{identity:'unidentified'},association:{item_id:item.id,specimen_id:null,component_id:null,carrier_id:null},expected_item_revision:item.revision})});
  const accepted=await request(`/imports/${reviewed.id}/publish`,{method:'POST'});
  for(let n=0;n<1200;n++){const job=await request('/jobs/'+accepted.job_id);if(job.state==='completed')return {item:id,collection:collection.id,capture:reviewed.capture_id,representation:reviewed.representation_id};if(job.state==='failed')throw new Error(job.error);await new Promise(r=>setTimeout(r,100));}throw new Error('publication timed out');
 });
 await page.goto(origin+'/?collection='+fixture.collection+'&item='+fixture.item);await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();
 const editor=page.getByRole('dialog',{name:/Edit entry/});const scans=editor.getByRole('region',{name:'Game package analysis'});const docket=scans.locator('.import-docket');
 await docket.getByLabel('Operation').selectOption('org.junk.games.hash');await docket.getByRole('button',{name:'Run operation',exact:true}).waitFor();
 const accepted=page.waitForResponse(r=>new URL(r.url()).pathname===`/api/v1/captures/${fixture.capture}/operations`&&r.request().method()==='POST');
 await docket.getByRole('button',{name:'Run operation',exact:true}).click();const job=(await(await accepted).json()).job_id;
 await docket.getByRole('button',{name:'Check in progress…',exact:true}).waitFor();assert.equal(await docket.getByRole('button',{name:'Run operation',exact:true}).count(),0);
 assert.equal(await editor.getByLabel('Title',{exact:true}).isEnabled(),true,'scan locked unrelated editor fields');
 const joined=await page.evaluate(async f=>{
  const body={operation:'org.junk.games.hash',operation_version:1,representation_id:f.representation,snapshot_id:null,selection:{profile:'raw'}};
  return Promise.all([1,2].map(async()=>{const response=await fetch(`/api/v1/captures/${f.capture}/operations`,{method:'POST',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify(body)});if(!response.ok)throw new Error(await response.text());return response.json();}));
 },fixture);assert.ok(joined.every(value=>value.job_id===job));
 await editor.getByRole('button',{name:'Close curator'}).click();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();await docket.getByRole('button',{name:'Check in progress…',exact:true}).waitFor();
 await docket.getByRole('button',{name:'View result',exact:true}).waitFor({timeout:120000});assert.equal(await docket.getByRole('button',{name:'Run operation',exact:true}).count(),0);assert.match(await docket.innerText(),/Checked 1 time/);
 await editor.getByRole('button',{name:'Close curator'}).click();await page.reload();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();await docket.getByRole('button',{name:'View result',exact:true}).waitFor();
 await docket.getByText('More actions',{exact:true}).click();await docket.getByRole('button',{name:'Check again',exact:true}).click();await docket.getByRole('button',{name:'Check in progress…',exact:true}).waitFor();await docket.getByText('Checked 2 times',{exact:false}).waitFor({timeout:120000});assert.equal(await docket.locator('.stored-file').count(),1);
 await page.route('**/operations/plan',route=>route.fulfill({status:503,json:{error:{code:'temporarily_unavailable',message:'Status temporarily unavailable'}}}),{times:1});
 await docket.getByLabel('Operation').selectOption('org.junk.games.hash-tracks');await docket.getByRole('button',{name:'Retry check status'}).click();await docket.getByText('No eligible package file is available.',{exact:true}).waitFor();assert.equal(await docket.getByRole('button',{name:'Run operation',exact:true}).isEnabled(),false);
 const invalid=await page.evaluate(async f=>{const before=await(await fetch('/api/v1/jobs?limit=100')).json();const response=await fetch(`/api/v1/captures/${f.capture}/operations`,{method:'POST',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({operation:'org.junk.games.hash-tracks',operation_version:1,representation_id:f.representation,snapshot_id:null,selection:{cue_file_index:0}})});const after=await(await fetch('/api/v1/jobs?limit=100')).json();return {status:response.status,before:before.items.length,after:after.items.length};},fixture);assert.equal(invalid.status,400);assert.equal(invalid.before,invalid.after);
 assert.deepEqual(errors,[]);console.log(`FX-02 browser journey passed (${await browser.version()}): real 64 MiB scan, concurrent admission, reopen, repeat consolidation, unrelated editor controls, invalid input without a job.`);
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot());await page.screenshot({path:'/tmp/fx02-browser-failure.png',fullPage:true});}throw error;}finally{
 if(browser)await browser.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(r=>server.once('exit',r)),new Promise(r=>setTimeout(()=>{server.kill('SIGKILL');r();},5000))]);}await rm(scratch,{recursive:true,force:true});
}
