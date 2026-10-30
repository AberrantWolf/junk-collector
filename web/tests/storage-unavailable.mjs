// Real UI, real storage guard, owned temporary directories. No NFS/production I/O.
import {chromium,expect} from './support/browser.mjs';
import {spawn,spawnSync,stopProcess,waitForServer,serverBinary} from './support/runtime.mjs';
import {mkdtemp,writeFile,rename,stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';

const scratch=await mkdtemp(path.join(tmpdir(),'collector-storage-availability-'));
const config=path.join(scratch,'collector.toml'),archive=path.join(scratch,'archive'),detached=path.join(scratch,'detached-archive');
const origin='http://localhost:20152',password=randomBytes(24).toString('hex');
await writeFile(config,`archive = "${archive}"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:20151"\nprivate_bind = "127.0.0.1:20152"\nlan_bind = "127.0.0.1:20153"\nlocal_playback_enabled = false\nauthentication_origin = "https://localhost:20153"\nallowed_origins = ["${origin}"]\n`);
const cli=(command,input)=>spawnSync(serverBinary,['--config',config,command],{input,encoding:'utf8',timeout:20000});
const setup=cli('setup',password);assert.equal(setup.status,0,setup.stderr);
let server,browser,page,logs='';
async function start(){server=spawn(serverBinary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stdout.on('data',v=>logs+=v);server.stderr.on('data',v=>logs+=v);await waitForServer('http://127.0.0.1:20151/healthz',server,{logs:()=>logs});}
try{
 await start();browser=await chromium.launch({headless:true});page=await browser.newPage({viewport:{width:1200,height:900}});page.setDefaultTimeout(15000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Retained collection');await admin.getByLabel('Collection type').selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await page.getByRole('button',{name:'Close administration'}).click();
 const id=await page.evaluate(async()=>{
  const collection=(await(await fetch('/api/v1/collection-search?limit=50')).json()).items.find(c=>c.name==='Retained collection');const id=crypto.randomUUID();
  const response=await fetch('/api/v1/items/'+id,{method:'PUT',headers:{'Content-Type':'application/json','Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({expected_revision:0,edit:{collection_id:collection.id,title:'Retained manual album',subtitle:'Storage recovery fixture',identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{artist:'Retained artist'}},identity:'unidentified',translations:[],identifiers:[],wishlist:true}})});
  if(!response.ok)throw Error(await response.text());return id;
 });
 await page.reload();await expect(page.locator(`[data-item-id="${id}"]`)).toContainText('Retained manual album');
 // Only this fixture's newly initialized archive is detached.
 await stat(path.join(archive,'.archive-identity'));await rename(archive,detached);
 const failed=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/v1/me');await page.reload();const response=await failed;assert.equal(response.status(),503);assert.equal((await response.json()).error.code,'unavailable');
 const alert=page.getByRole('alert').filter({hasText:/storage.*unavailable/i});await expect(alert).toBeVisible();assert.equal(await page.getByText('No collections yet',{exact:true}).count(),0);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'storage error overflows mobile');
 await rename(detached,archive);
 assert.equal(await page.evaluate(async()=> (await fetch('/api/v1/me')).status),503,'reconnect silently cleared the process latch');
 await stopProcess(server);await rename(archive,detached);const refused=cli('serve');assert.notEqual(refused.status,0);assert.notEqual(refused.status,null,'missing-storage startup hung');await assert.rejects(stat(archive),{code:'ENOENT'});
 await rename(detached,archive);await start();await page.reload();await expect(page.locator(`[data-item-id="${id}"]`)).toContainText('Retained manual album');assert.deepEqual(errors,[]);
 console.log('Storage availability UI passed: visible typed 503, mobile error wrapping, latched reconnect, refused missing-root startup and retained collection/manual title after restart.');
}catch(error){console.error(logs);console.error(await page?.locator('body').innerText({timeout:2000}).catch(()=>''));throw error;}finally{await browser?.close();await stopProcess(server);}
