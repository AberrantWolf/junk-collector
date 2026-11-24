// Isolated UI journey through the actual server, shared browser harness and opusenc.
import {chromium,expect} from './support/browser.mjs';
import {root,serverBinary,spawn,spawnSync,stopProcess,waitForServer,fixturePort} from './support/runtime.mjs';
import {mkdtemp,writeFile,readFile,rm,unlink} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,randomUUID,createHash} from 'node:crypto';
import assert from 'node:assert/strict';

async function includeAllAlbumFiles(imports){
 await imports.locator('summary').filter({hasText:'Files and album playback'}).click();
 await imports.getByLabel('Use folder for album playback: All files',{exact:true}).check();
}
const opus=process.env.COLLECTOR_OPUS||spawnSync('which',['opusenc'],{encoding:'utf8'}).stdout?.trim();
assert.ok(opus&&spawnSync(opus,['--version']).status===0,'Opus journey requires COLLECTOR_OPUS or opusenc on PATH');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-opus-'));
const password=randomBytes(24).toString('hex');
let server,browser,page,logs='';
async function request(page,route,body,method='POST'){
 return page.evaluate(async ({route,body,method})=>{
  const response=await fetch('/api/v1'+route,body===undefined?{}:{method,headers:{'Content-Type':'application/json','Idempotency-Key':crypto.randomUUID()},body:JSON.stringify(body)});
  if(!response.ok)throw Error(await response.text());
  return response.json();
 },{route,body,method});
}
async function setDefaultBitrate(page,collection,bitrate_kbps,scope='cd_and_digital'){
 const path=`/collections/${collection}/cd-settings`;
 const saved=await request(page,path);
 await request(page,path,{
  expected_revision:saved.revision,check_accuraterip_on_ingest:false,maximum_offset:saved.maximum_offset,
  opus_import:{scope,bitrate_kbps},
 },'PUT');
}
async function reviewImport(imports,files){
 await imports.getByLabel("Choose one package's files").setInputFiles(files);
 await expect(imports.getByRole('button',{name:'Start import',exact:true})).toBeVisible();
 await imports.getByLabel('Automatic metadata and artwork').uncheck();
 return imports.getByLabel('Audio preparation after import');
}
async function bulk(page,id){
 const summary=await request(page,`/bulk-library/${id}`);
 return {state:summary.state,succeeded:summary.succeeded,failed:summary.failed,pending:summary.pending,tracks:summary.tracks,already_ready:summary.already_ready};
}
async function readiness(page,collection,bitrate,item){
 const rows=(await request(page,`/library-care/music-preparation?collection_id=${collection}&needs_attention=true&limit=50&opus_bitrate_kbps=${bitrate}`)).items;
 const row=rows.find(row=>row.item.id===item);
 return row?.readiness?{tracks:row.readiness.tracks,ready:row.readiness.ready}:row?'pending':'absent';
}
async function preparationRows(page,collection,bitrate){
 return (await request(page,`/library-care/music-preparation?collection_id=${collection}&needs_attention=true&limit=50&opus_bitrate_kbps=${bitrate}`)).items.length;
}
async function startImport(page,imports){
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));
 return new URL(page.url()).searchParams.get('item');
}
async function member(page,intake){
 return (await request(page,`/intakes/${intake}/members?limit=1`)).items[0]?.import;
}
async function outcome(page,intake){
 const summary=await member(page,intake);
 if(summary?.outcome==='failed'){
  const detail=await request(page,`/ingests/${summary.import_id}`);
  throw Error(JSON.stringify(detail.steps));
 }
 return summary?.outcome;
}
try{
 const privatePort=await fixturePort(),origin=`http://localhost:${privatePort}`;
 const config=path.join(scratch,'collector.toml');
 const gate=path.join(scratch,'allow-encoding');
 const refuse=path.join(scratch,'refuse-encoding');
 const wrapper=path.join(scratch,'opusenc');
 await writeFile(wrapper,`#!/bin/sh
if [ "$1" != "--version" ]; then
 if [ -f '${refuse}' ]; then echo 'fixture encoder refusal' >&2; exit 7; fi
 n=0
 while [ ! -f '${gate}' ] && [ "$n" -lt 200 ]; do n=$((n+1)); sleep 0.1; done
fi
exec "$COLLECTOR_OPUS_FIXTURE" "$@"
`,{mode:0o755});
 await writeFile(config,`archive = "${scratch}/archive"
state = "${scratch}/state"
public_bind = "127.0.0.1:${await fixturePort()}"
private_bind = "127.0.0.1:${privatePort}"
lan_bind = "127.0.0.1:${await fixturePort()}"
local_playback_enabled = false
allowed_origins = ["${origin}"]
[encoders]
opus = "${wrapper}"
`);
 const env={...process.env,COLLECTOR_OPUS_FIXTURE:opus};
 const setup=spawnSync(serverBinary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8',env});
 assert.equal(setup.status,0,setup.stderr);
 server=spawn(serverBinary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe'],env});
 server.stdout.on('data',value=>logs+=value);server.stderr.on('data',value=>logs+=value);
 await waitForServer(origin+'/healthz',server,{logs:()=>logs});
 browser=await chromium.launch();
 const context=await browser.newContext({viewport:{width:1280,height:920}});
 page=await context.newPage();
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);
 await page.getByLabel('Username',{exact:true}).fill('owner');
 await page.getByLabel('Password',{exact:true}).fill(password);
 await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Opus fixture');
 await admin.getByLabel('Collection type').selectOption('org.junk.music');
 await admin.getByRole('button',{name:'Create collection',exact:true}).click();
 await expect.poll(async()=>(await request(page,'/collection-search?limit=50')).items.some(c=>c.name==='Opus fixture')).toBe(true);
 await expect(admin.getByRole('alert')).toHaveCount(0);
 await admin.getByRole('button',{name:'Library settings',exact:true}).click();
 await admin.getByLabel('Check CD rips with AccurateRip on import').uncheck();
 await admin.getByLabel('Automatic Opus preparation').selectOption('cd_and_digital');
 await admin.getByLabel('Default Opus bitrate').selectOption('96');
 await admin.getByRole('button',{name:'Save CD defaults',exact:true}).click();
 await expect(admin.getByRole('region',{name:'CD library defaults'}).getByRole('status')).toBeVisible();
 const collection=(await request(page,'/collection-search?limit=50')).items.find(c=>c.name==='Opus fixture').id;
 await admin.getByRole('button',{name:'Close administration'}).click();

 let detailReads=0;
 page.on('request',request=>{
  if(/\/api\/v1\/items\/[^/?]+(?:\?|$|\/music-preparation-plan)/.test(request.url()))detailReads++;
 });
 await page.goto(`${origin}/library-care/music-preparation?collection=${collection}&opus_bitrate_kbps=128`);
 const preparation=page.getByRole('region',{name:'Prepare audio workspace'});
 await expect.poll(()=>preparationRows(page,collection,128)).toBe(0);
 await expect(preparation.getByRole('button',{name:'Select all matching entries'})).toBeVisible();
 await expect(preparation.locator('[data-item-id]')).toHaveCount(0);
 const intakePage=await context.newPage();
 intakePage.on('pageerror',error=>errors.push(error.message));
 await intakePage.goto(`${origin}/imports?collection=${collection}`);
 const imports=intakePage.getByRole('region',{name:'Imports workspace'});
 const audio=['01.flac','02.mp3','03.wav'].map(file=>path.join(root,'crates/collector-store/tests/fixtures/digital-audio',file));
 const defaultChoice=await reviewImport(imports,audio);
 await includeAllAlbumFiles(imports);
 await expect(defaultChoice).toHaveValue('default');
 await setDefaultBitrate(page,collection,128,'cd');
 const digitalIntake=await startImport(intakePage,imports);
 await expect.poll(async()=>{
  const summary=await member(intakePage,digitalIntake);
  return [summary?.phase,summary?.outcome,summary?.preparation_profile?.bitrate_kbps];
 },{timeout:30000}).toEqual(['published','active',96]);
 const digital=(await member(intakePage,digitalIntake)).target_item_id;
 const digitalRow=preparation.locator(`[data-item-id="${digital}"]`);
 await expect(digitalRow).toBeVisible();
 await expect.poll(()=>readiness(page,collection,128,digital)).toEqual({tracks:3,ready:0});

 // Change the default while the preserved digital import is still preparing.
 await setDefaultBitrate(intakePage,collection,192);
 await writeFile(gate,'ready');
 await expect.poll(async()=> outcome(intakePage,digitalIntake),{timeout:45000}).toBe('succeeded');
 const item=await request(intakePage,`/items/${digital}`);
 assert.equal(item.specimens.length,0);
 const original=item.captures[0].representations.find(r=>r.role==='preservation_master');
 const prepared=item.captures[0].representations.filter(r=>r.profile?.codec==='opus');
 assert.equal(prepared.length,3);
 assert.ok(prepared.every(r=>r.profile.bitrate_kbps===96));
 for(const [index,file] of audio.entries()){
  assert.equal(original.files[index].digest.hex,createHash('sha256').update(await readFile(file)).digest('hex'));
 }

 // Import a CD without its default and verify the open list sees the new album.
 const cue=path.join(scratch,'Opus CD.cue'),bin=path.join(scratch,'disc.bin');
 await writeFile(cue,'FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n');
 await writeFile(bin,Buffer.alloc(225*2352,1));
 await imports.getByRole('button',{name:'Back to Add to library',exact:true}).click();
 const override=await reviewImport(imports,[cue,bin]);
 await expect(override).toHaveValue('default');
 await override.selectOption('none');
 const cdIntake=await startImport(intakePage,imports);
 await expect.poll(async()=> outcome(intakePage,cdIntake),{timeout:45000}).toBe('succeeded');
 const cdImport=await member(intakePage,cdIntake);
 assert.equal(cdImport.preparation_profile,null);
 const cdRow=preparation.locator(`[data-item-id="${cdImport.target_item_id}"]`);
 await expect(cdRow).toBeVisible();
 await expect.poll(()=>readiness(page,collection,128,cdImport.target_item_id)).toEqual({tracks:1,ready:0});

 const savedCd=await request(intakePage,`/items/${cdImport.target_item_id}`);
 await request(intakePage,`/items/${savedCd.id}`,{
  expected_revision:savedCd.revision,edit:{collection_id:collection,title:'Renamed CD fixture',subtitle:'',
   identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{}},
   identity:'unidentified',translations:[],identifiers:[],wishlist:false},
 },'PUT');
 await expect(cdRow).toContainText('Renamed CD fixture');

 const bitrate=preparation.getByLabel('Opus bitrate',{exact:true});
 await bitrate.selectOption('96');
 await expect(cdRow).toBeVisible();await expect(digitalRow).toHaveCount(0);
 const filtered=await request(page,`/library-care/music-preparation?collection_id=${collection}&needs_attention=true&limit=1&opus_bitrate_kbps=96`);
 assert.deepEqual(filtered.items.map(row=>row.item.id),[cdImport.target_item_id]);
 assert.equal(filtered.next_cursor,null);
 await bitrate.selectOption('128');
 await expect(digitalRow).toBeVisible();await expect(cdRow).toBeVisible();
 assert.equal(detailReads,0,'Prepare audio list opened album detail or plans');
 await preparation.getByRole('button',{name:'Select all matching entries'}).click();
 await writeFile(refuse,'refuse');
 const accepted=page.waitForResponse(response=>response.url().endsWith('/api/v1/bulk-library/music-preparation')&&response.request().method()==='POST');
 await preparation.getByRole('button',{name:'Start Opus 128 kbps preparation'}).click();
 const queued=await (await accepted).json();
 assert.equal(queued.selected,2);
 assert.equal(queued.tracks,0,'Submission should return before track planning');
 const batch=preparation.getByRole('region',{name:'Review batch'});
 await batch.waitFor();
 await expect.poll(async()=>{const b=await bulk(page,queued.id);return [b.succeeded,b.failed,b.pending];},{timeout:45000}).toEqual([0,2,0]);
 await expect.poll(async()=>{const b=await bulk(page,queued.id);return [b.tracks,b.already_ready];}).toEqual([4,0]);
 const failedBatch=(await request(page,`/items/${digital}/music-preparation-batches`)).find(batch=>batch.profile.bitrate_kbps===128);
 assert.ok(failedBatch.members.every(track=>/opusenc conversion failed: fixture encoder refusal/.test(track.error)));
 await unlink(refuse);
 await batch.getByRole('button',{name:'Retry unfinished tasks'}).click();
 await expect.poll(async()=>{const b=await bulk(page,queued.id);return [b.succeeded,b.failed,b.pending];},{timeout:45000}).toEqual([2,0,0]);
 await expect(batch.getByRole('button',{name:'Retry unfinished tasks'})).toHaveCount(0);
 await preparation.getByRole('button',{name:'Back to preparation results'}).click();
 await expect.poll(()=>preparationRows(page,collection,128)).toBe(0);
 await expect(preparation.locator('[data-item-id]')).toHaveCount(0);

 // A different preset stays independently eligible in the single-album workflow.
 await page.goto(`${origin}/activity?collection=${collection}&batch=${queued.id}`);
 await page.getByRole('region',{name:'Review batch'}).getByRole('button',{name:'Open album preparation'}).first().click();
 await preparation.getByRole('region',{name:'Album preparation'}).waitFor();
 await expect(preparation.getByRole('region',{name:'Prepare album'})).toHaveCount(0);
 await expect(preparation.getByRole('region',{name:'Album preparation'}).locator('.notice[role=status]')).toBeVisible();
 for(const id of [digital,cdImport.target_item_id]){const reps=(await request(page,`/items/${id}`)).captures.flatMap(c=>c.representations).filter(r=>r.profile?.codec==='opus'&&r.profile.bitrate_kbps===128);assert.ok(reps.length>0,'128 kbps output missing');}
 await preparation.getByRole('button',{name:'Back to preparation results'}).click();
 await bitrate.selectOption('192');
 await unlink(gate);
 await preparation.getByRole('checkbox',{name:'Select Renamed CD fixture'}).check();
 const cancelAccepted=page.waitForResponse(response=>response.url().endsWith('/api/v1/bulk-library/music-preparation')&&response.request().method()==='POST');
 await preparation.getByRole('button',{name:'Start Opus 192 kbps preparation'}).click();
 const cancelled=await (await cancelAccepted).json();
 await expect.poll(async()=>{const b=await bulk(page,cancelled.id);return [b.tracks,b.already_ready];},{timeout:45000}).toEqual([1,0]);
 await batch.getByRole('button',{name:'Cancel remaining work'}).click();
 await expect.poll(async()=>(await bulk(page,cancelled.id)).state,{timeout:45000}).toBe('cancelled');
 await expect(batch.getByRole('button',{name:'Cancel remaining work'})).toHaveCount(0);
 await preparation.getByRole('button',{name:'Back to preparation results'}).click();
 await writeFile(gate,'ready');
 await digitalRow.click();
 const album=preparation.getByRole('region',{name:'Album preparation'});
 await expect(album.getByRole('region',{name:'Prepare album'})).toBeVisible();
 await unlink(gate);
 await album.getByRole('button',{name:'Prepare missing Opus 192 kbps tracks'}).click();
 await album.getByRole('button',{name:'Cancel outstanding tracks'}).click();
 await expect.poll(async()=>(await request(page,`/items/${digital}/music-preparation-batches`)).find(batch=>batch.profile.bitrate_kbps===192)?.cancelled_by_user,{timeout:45000}).toBe(true);
 await expect(album.getByRole('region',{name:'Album preparation progress'})).toBeVisible();
 await writeFile(gate,'ready');
 const retryResponse=page.waitForResponse(response=>
  /\/api\/v1\/music-preparation-batches\/[^/]+\/retry$/.test(response.url())
  &&response.request().method()==='POST');
 await album.getByRole('button',{name:'Retry unfinished tracks'}).click();
 assert.equal((await retryResponse).status(),200,'Retry must be accepted after cancellation settles');
 let opus192=[];
 await expect.poll(async()=>{opus192=(await request(page,`/items/${digital}`)).captures.flatMap(c=>c.representations).filter(r=>r.profile?.codec==='opus'&&r.profile.bitrate_kbps===192).map(r=>r.id);return opus192.length;},{timeout:45000}).toBe(3);
 await expect(album.getByRole('region',{name:'Prepare album'})).toHaveCount(0,{timeout:45000});
 const opusLink=album.locator(opus192.map(id=>`a[href^="/api/v1/items/${digital}/representations/${id}/"]`).join(',')).first();
 await expect(opusLink).toBeVisible();
 const href=await opusLink.getAttribute('href');
 const magic=await page.evaluate(async href=>{
  const response=await fetch(href,{headers:{Range:'bytes=0-3'}});
  return [response.status,await response.text()];
 },href);
 assert.deepEqual(magic,[206,'OggS']);
 // A file whose declared format disagrees with its bytes remains preserved.
 const broken=path.join(scratch,'unsupported.mp3');
 const brokenBytes=await readFile(audio[0]);
 await writeFile(broken,brokenBytes);
 await intakePage.goto(`${origin}/imports?collection=${collection}`);
 await reviewImport(imports,[broken]);
 const brokenIntake=await startImport(intakePage,imports);
 await expect.poll(async()=>{
  const summary=await member(intakePage,brokenIntake);
  return [summary?.phase,summary?.outcome];
 },{timeout:45000}).toEqual(['published','failed']);
 const brokenSummary=await member(intakePage,brokenIntake);
 const failed=await request(intakePage,`/ingests/${brokenSummary.import_id}`);
 const preparationFailure=failed.steps.find(step=>step.step==='preparation');
 assert.equal(preparationFailure.state,'failed');
 assert.equal(preparationFailure.job_id??null,null,'Unsupported bytes must fail before scheduling preparation');
 const retained=await request(intakePage,`/items/${brokenSummary.target_item_id}`);
 assert.equal(retained.captures[0].representations.length,1);
 assert.equal(retained.captures[0].representations[0].files[0].digest.hex,createHash('sha256').update(brokenBytes).digest('hex'));

 // Select all from the first page. Only compact membership is submitted;
 // albums without originals must surface an actionable background outcome.
 const scaleAlbums=55;
 for(let index=0;index<scaleAlbums;index++){
  await request(intakePage,`/items/${randomUUID()}`,{
   expected_revision:0,edit:{collection_id:collection,title:`Scale fixture ${String(index).padStart(2,'0')}`,subtitle:'',
    identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{}},
    identity:'unidentified',translations:[],identifiers:[],wishlist:false},
  },'PUT');
 }
 await page.goto(`${origin}/library-care/music-preparation?collection=${collection}&opus_bitrate_kbps=128&search=Scale%20fixture`);
 const selectAll=preparation.getByRole('button',{name:'Select all matching entries'});
 await expect(selectAll).toBeEnabled({timeout:45000});
 await expect(preparation.getByRole('checkbox',{name:/^Select Scale fixture/})).toHaveCount(50);
 await selectAll.click();
 await preparation.getByRole('checkbox',{name:'Select Scale fixture 00'}).uncheck();
 const scaleResponse=page.waitForResponse(response=>response.url().endsWith('/api/v1/bulk-library/music-preparation')&&response.request().method()==='POST');
 await preparation.getByRole('button',{name:'Start Opus 128 kbps preparation'}).click();
 const response=await scaleResponse;
 const scale=await response.json();
 assert.equal(scale.selected,scaleAlbums-1);
 assert.equal(scale.tracks,0);
 const scaleBatch=preparation.getByRole('region',{name:'Review batch'});
 await scaleBatch.waitFor();
 await expect.poll(async()=>{const b=await bulk(page,scale.id);return [b.failed,b.pending];},{timeout:45000}).toEqual([54,0]);
 await scaleBatch.getByRole('button',{name:'Next album results'}).click();
 await expect(scaleBatch.getByRole('button',{name:'Open album preparation'})).toHaveCount(4);
 await scaleBatch.getByRole('button',{name:'Previous album results'}).click();
 await expect(scaleBatch.getByRole('button',{name:'Open album preparation'})).toHaveCount(50);
 console.log(`54-album Start request reached response headers in ${Math.round(response.request().timing().responseStart)} ms (local fixture).`);

 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 assert.deepEqual(errors,[]);
 console.log('Opus UI passed: persisted digital default, override, active import, exact originals, live cached list, bitrate filters, two-album encoding batch, 54-album submission and downloadable single-album output.');
}catch(error){
 console.error(logs);
 if(page)console.error(await page.locator('body').innerText({timeout:2000}).catch(()=>''));
 throw error;
}finally{
 await browser?.close();
 await stopProcess(server);
 await rm(scratch,{recursive:true,force:true});
}
