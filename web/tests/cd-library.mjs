// Real UI + HTTP + durable jobs, with only the remote AccurateRip database fixture replaced.
import {chromium,expect} from './support/browser.mjs';
import {spawn,spawnSync,serverBinary,stopProcess,waitForServer} from './support/runtime.mjs';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {createServer} from 'node:http';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
const scratch=await mkdtemp(path.join(tmpdir(),'collector-cd-library-')),password=randomBytes(24).toString('hex');
let server,browser,page,logs='',providerRequests=0;
async function port(){const listener=createServer();await new Promise(r=>listener.listen(0,'127.0.0.1',r));const p=listener.address().port;await new Promise(r=>listener.close(r));return p;}
const frames=451*588,pcm=Buffer.alloc(frames*4);for(let i=0;i<frames;i++)pcm.writeInt16LE(1,i*4);
const checksum=Number(BigInt(2940+frames-2940)*BigInt(frames-5879)/2n)&0xffffffff;
const dbar=Buffer.alloc(22);dbar[0]=1;dbar.writeUInt32LE(451,1);dbar.writeUInt32LE(903,5);dbar.writeUInt32LE(0x02000601,9);dbar[13]=17;dbar.writeUInt32LE(checksum>>>0,14);
const provider=createServer((req,res)=>{providerRequests++;res.setHeader('content-type','application/octet-stream');res.end(dbar);});await new Promise(r=>provider.listen(0,'127.0.0.1',r));
try{
 const publicPort=await port(),privatePort=await port(),lanPort=await port(),origin=`http://localhost:${privatePort}`,config=path.join(scratch,'collector.toml');
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:${publicPort}"\nprivate_bind = "127.0.0.1:${privatePort}"\nlan_bind = "127.0.0.1:${lanPort}"\nlocal_playback_enabled = false\nallowed_origins = ["${origin}"]\n`);
 const setup=spawnSync(serverBinary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
 server=spawn(serverBinary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe'],env:{...process.env,JUNK_COLLECTOR_TEST_ACCURATERIP_ORIGIN:`http://127.0.0.1:${provider.address().port}/`}});server.stdout.on('data',v=>logs+=v);server.stderr.on('data',v=>logs+=v);await waitForServer(origin+'/healthz',server,{logs:()=>logs});
 browser=await chromium.launch();page=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Create collection',exact:true}).click();
 const admin=page.getByRole('dialog',{name:'Administration'});await admin.getByLabel('Collection name').fill('CD workflow fixture');await admin.getByLabel('Collection type').selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Library settings',exact:true}).click();
 const defaults=admin.getByRole('region',{name:'CD library defaults',exact:true});await defaults.getByLabel('Check CD audio with AccurateRip during ingest').uncheck();await defaults.getByLabel('Maximum offset (stereo sample frames)').fill('0');await defaults.getByRole('button',{name:'Save CD defaults',exact:true}).click();await expect(defaults.getByRole('status')).toContainText('CD defaults saved');
 // Save an older uninspected original through the real ingest API.
 const imported=await page.evaluate(async({bytes})=>{
  const call=async(url,body,method='POST')=>{const r=await fetch('/api/v1'+url,{method,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify(body)});if(!r.ok)throw Error(await r.text());return r.json();};
  const collection=(await(await fetch('/api/v1/collection-search?limit=100')).json()).items.find(c=>c.name==='CD workflow fixture');
  const cue='FILE "disc.bin" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n';
  const plan=await call('/ingests/plan',{collection_id:collection.id,packages:[{source:{kind:'browser',label:null,package_name:'Older CD'},files:[{name:'disc.cue',bytes:cue.length,sha256:null},{name:'disc.bin',bytes,sha256:null}],anchor_file_index:null,purchase:null}]});
  const accepted=await call('/ingests',{plan,analysis:'preserve_only',enrichment:'none',prepare_music:false,purchase:null,clean_reimport:null,destination:null});const id=accepted.imports[0].import_id;
  for(const [index,body] of [[0,new TextEncoder().encode(cue)],[1,(()=>{const b=new Uint8Array(bytes);for(let i=0;i<bytes;i+=4)b[i]=1;return b;})()]]){const r=await fetch(`/api/v1/imports/${id}/files/${index}?offset=0`,{method:'PUT',headers:{'content-type':'application/octet-stream'},body});if(!r.ok)throw Error(await r.text());}
  return accepted.imports[0];
 },{bytes:pcm.length});
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch('/api/v1/ingests/'+id)).json()).import.phase,imported.import_id)).toBe('published');assert.equal(providerRequests,0);
 // More than two pages of non-CD entries must not crowd the actual check results.
 await page.evaluate(async collection=>{
  for(let index=0;index<104;index++){
   const response=await fetch('/api/v1/items/'+crypto.randomUUID(),{method:'PUT',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({expected_revision:0,edit:{collection_id:collection,title:`Excluded entry ${String(index).padStart(3,'0')}`,subtitle:'',identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{}},identity:'unidentified',translations:[],identifiers:[],wishlist:false}})});
   if(!response.ok)throw Error(await response.text());
  }
 },imported.collection_id??await page.evaluate(async()=>((await(await fetch('/api/v1/collection-search?limit=100')).json()).items.find(c=>c.name==='CD workflow fixture').id)));
 await defaults.getByRole('button',{name:'Check unconfirmed rips',exact:true}).click();const batch=defaults.getByRole('region',{name:'Review batch',exact:true});await expect(batch.getByText('105 selected albums',{exact:false})).toBeVisible();await expect(batch.getByText('1 successful',{exact:false})).toBeVisible({timeout:30000});assert.equal(providerRequests,1);
 await expect(batch.getByText('104 excluded',{exact:false})).toBeVisible();
 await expect(batch.locator('.care-rows .care-row')).toHaveCount(1);
 await expect(batch.locator('.care-rows')).toContainText('Older CD');
 await expect(batch.locator('.care-rows')).not.toContainText('Excluded entry');
 await expect(batch.getByRole('button',{name:'More album results',exact:true})).toHaveCount(0);
 // A later check has only exclusions, including the newly confirmed CD.
 await defaults.getByRole('button',{name:'Check unconfirmed rips',exact:true}).click();
 await expect(batch.getByText('105 excluded',{exact:false})).toBeVisible();
 await expect(batch.locator('.care-rows .care-row')).toHaveCount(0);
 await expect(batch.getByRole('button',{name:'More album results',exact:true})).toHaveCount(0);
 assert.equal(providerRequests,1,'excluded confirmed audio was checked again');
 await admin.getByRole('button',{name:'Close administration'}).click();await page.reload();await page.getByRole('searchbox',{name:'Search collection'}).fill('Older CD');await expect(page.locator(`[data-item-id="${imported.target_item_id}"]`)).toBeVisible();await expect(page.getByRole('button',{name:/AccurateRip for Older CD: Verified\. Open audio checks/})).toBeVisible();
 assert.deepEqual(errors,[]);console.log('CD library UI passed: saved collection defaults, preserved uninspected original without acquisition, one-click library batch, automatic structure/response/audio confirmation, refreshed verified library row, and no excluded rows or empty result pages across 104 skipped entries.');
}catch(e){console.error(logs);if(page)console.error(await page.locator('body').innerText({timeout:2000}).catch(()=>''));throw e;}finally{await browser?.close();await stopProcess(server);await new Promise(r=>provider.close(r));await rm(scratch,{recursive:true,force:true});}
