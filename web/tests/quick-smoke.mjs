// Short real-server acceptance; no provider, encoder or restart fixtures.
import {chromium,expect,fixtureCertificateArgs} from './support/browser.mjs';
import {spawn,spawnSync,serverBinary,stopProcess,waitForServer} from './support/runtime.mjs';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {createServer} from 'node:net';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
import {clickAction} from './actions.mjs';
const started=Date.now();
let scratch,server,browser,page,logs='';
const failedRequests=[];
let origin=process.env.COLLECTOR_TEST_ORIGIN;
const ignoreHTTPSErrors=process.env.COLLECTOR_TEST_SELF_SIGNED==='1';
const certificate=process.env.COLLECTOR_TEST_CERTIFICATE_PEM;
if(ignoreHTTPSErrors&&!certificate)throw Error('Self-signed smoke requires the disposable fixture certificate in COLLECTOR_TEST_CERTIFICATE_PEM.');
const password=process.env.COLLECTOR_TEST_PASSWORD||randomBytes(24).toString('hex');
async function port(){const listener=createServer();await new Promise(resolve=>listener.listen(0,'127.0.0.1',resolve));const value=listener.address().port;await new Promise(resolve=>listener.close(resolve));return value;}
try{
 if(!origin){
  scratch=await mkdtemp(path.join(tmpdir(),'collector-quick-smoke-'));
  const publicPort=await port(),privatePort=await port(),lanPort=await port();origin=`http://localhost:${privatePort}`;
  const config=path.join(scratch,'collector.toml');
  await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:${publicPort}"\nprivate_bind = "127.0.0.1:${privatePort}"\nlan_bind = "127.0.0.1:${lanPort}"\nlocal_playback_enabled = false\nmulti_user = true\nallowed_origins = ["${origin}"]\n`);
  const setup=spawnSync(serverBinary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
  server=spawn(serverBinary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',value=>logs+=value);server.stdout.on('data',value=>logs+=value);
  await waitForServer(origin+'/healthz',server,{logs:()=>logs});
 }
 browser=await chromium.launch({args:certificate?fixtureCertificateArgs(certificate):[]});page=await browser.newPage({ignoreHTTPSErrors,viewport:{width:1440,height:1000}});page.setDefaultTimeout(15000);
 const errors=[];page.on('pageerror',error=>errors.push(error.message));page.on('requestfailed',request=>{if(failedRequests.length<30)failedRequests.push({url:request.url(),error:request.failure()?.errorText});});
 await page.goto(origin);
 for(const file of ['app.js','app.css','curation.js'])assert.equal(await page.evaluate(async file=>(await fetch('/'+file)).text(),file),await readFile(new URL('../dist/'+file,import.meta.url),'utf8'),`stale served asset: ${file}`);
 await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Smoke collection');await admin.getByLabel('Collection type').selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const fixture=await page.evaluate(async()=>{
  const call=async(url,body)=>{const response=await fetch('/api/v1'+url,{method:body?'POST':'GET',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},...body?{body:JSON.stringify(body)}:{}});if(!response.ok)throw Error(await response.text());return response.status===204?undefined:response.json();};
  const collection=(await call('/collection-search?limit=100')).items.find(c=>c.name==='Smoke collection');const id=crypto.randomUUID();
  const response=await fetch('/api/v1/items/'+id,{method:'PUT',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({expected_revision:0,edit:{collection_id:collection.id,title:'Smoke album',subtitle:'Short acceptance',identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{artist:'Fixture ensemble'}},identity:'unidentified',translations:[],identifiers:[],wishlist:false}})});if(!response.ok)throw Error(await response.text());
  const viewer=await call('/admin/users',{username:'smoke-viewer',password:'Disposable-smoke-viewer-password',administrator:false});
  await call('/admin/grants',{collection_id:collection.id,user_id:viewer.user_id,role:'viewer'});
  return {id,collection:collection.id};
 });
 await page.reload();await page.locator(`[data-item-id="${fixture.id}"]`).click();await clickAction(page,'Edit / attach files');const editor=page.getByRole('dialog');
 await editor.getByRole('button',{name:'Artwork',exact:true}).click();await editor.getByLabel('Artwork image').setInputFiles({name:'cover.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64')});await editor.getByRole('button',{name:'Attach image',exact:true}).click();await editor.getByText('Artwork and photos updated.',{exact:true}).waitFor();await editor.getByRole('button',{name:'Close curator',exact:true}).click();
 await page.getByRole('button',{name:'Back to results',exact:true}).click();await page.getByRole('searchbox',{name:'Search collection'}).fill('Smoke album');const row=page.locator(`[data-item-id="${fixture.id}"]`);await expect(row).toBeVisible();await expect(row.locator('img')).toBeVisible();await expect.poll(()=>row.locator('img').evaluate(image=>image.complete&&image.naturalWidth>0)).toBe(true);
 await row.click();await page.getByLabel('Smoke album details',{exact:true}).waitFor();
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile horizontal overflow');
 const results=process.env.COLLECTOR_SCREENSHOTS;if(results){await mkdir(results,{recursive:true});await page.screenshot({path:path.join(results,'smoke-mobile.png'),fullPage:true});await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:path.join(results,'smoke-desktop.png'),fullPage:true});}
 const viewer=await browser.newContext({ignoreHTTPSErrors});const denied=await viewer.newPage();await denied.goto(origin);await denied.getByLabel('Username',{exact:true}).fill('smoke-viewer');await denied.getByLabel('Password',{exact:true}).fill('Disposable-smoke-viewer-password');await denied.getByRole('button',{name:'Sign in',exact:true}).click();await denied.getByLabel('Password',{exact:true}).waitFor({state:'detached'});
 assert.equal(await denied.evaluate(async()=>{const response=await fetch('/api/v1/admin/users',{method:'POST',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({username:'forbidden',password:'Not-an-actual-user-password',administrator:false})});return response.status;}),403);
 assert.deepEqual(errors,[]);console.log(`Short smoke passed in ${((Date.now()-started)/1000).toFixed(2)}s: login, artwork/search/detail, management bundle, viewer denial, mobile layout and served assets.`);
} catch(error){console.error(logs);console.error('Failed requests:',failedRequests);if(page)console.error(await page.locator('body').innerText({timeout:2000}).catch(()=>''));throw error;}
finally{try{await browser?.close();}finally{await stopProcess(server);if(scratch)await rm(scratch,{recursive:true,force:true});}}
