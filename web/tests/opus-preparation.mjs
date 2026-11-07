// Isolated UI journey through the actual server, shared browser harness and opusenc.
import {chromium,expect} from './support/browser.mjs';
import {root,serverBinary,spawn,spawnSync,stopProcess,waitForServer,fixturePort} from './support/runtime.mjs';
import {mkdtemp,writeFile,readFile,rm,unlink} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,createHash} from 'node:crypto';
import assert from 'node:assert/strict';

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
 await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
 await imports.getByLabel('Automatic metadata and artwork').uncheck();
 return imports.getByLabel('Audio preparation after import');
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
 await admin.getByText('Collection created.',{exact:false}).waitFor();
 await admin.getByRole('button',{name:'Library settings',exact:true}).click();
 await admin.getByLabel('Check CD audio with AccurateRip during ingest').uncheck();
 await admin.getByLabel('Automatic Opus preparation').selectOption('cd_and_digital');
 await admin.getByLabel('Default Opus bitrate').selectOption('96');
 await admin.getByRole('button',{name:'Save CD defaults',exact:true}).click();
 await expect(admin.getByRole('region',{name:'CD library defaults'}).getByRole('status')).toContainText('CD defaults saved');
 const collection=(await request(page,'/collection-search?limit=50')).items.find(c=>c.name==='Opus fixture').id;
 await admin.getByRole('button',{name:'Close administration'}).click();

 let detailReads=0;
 page.on('request',request=>{
  if(/\/api\/v1\/items\/[^/?]+(?:\?|$|\/music-preparation-plan)/.test(request.url()))detailReads++;
 });
 await page.goto(`${origin}/library-care/music-preparation?collection=${collection}&opus_bitrate_kbps=128`);
 const preparation=page.getByRole('region',{name:'Prepare audio workspace'});
 await expect(preparation.getByText('No albums match these preparation filters.')).toBeVisible();
 const intakePage=await context.newPage();
 intakePage.on('pageerror',error=>errors.push(error.message));
 await intakePage.goto(`${origin}/imports?collection=${collection}`);
 const imports=intakePage.getByRole('region',{name:'Imports workspace'});
 const audio=['01.flac','02.mp3','03.wav'].map(file=>path.join(root,'crates/collector-store/tests/fixtures/digital-audio',file));
 const defaultChoice=await reviewImport(imports,audio);
 await expect(defaultChoice.locator('option[value=default]')).toContainText('Opus 96 kbps');
 await setDefaultBitrate(page,collection,128,'cd');
 const digitalIntake=await startImport(intakePage,imports);
 await expect.poll(async()=>{
  const summary=await member(intakePage,digitalIntake);
  return [summary?.phase,summary?.outcome,summary?.preparation_profile?.bitrate_kbps];
 },{timeout:30000}).toEqual(['published','active',96]);
 await expect(imports.getByText('Requested audio preparation · Opus 96 kbps')).toBeVisible();
 const digital=(await member(intakePage,digitalIntake)).target_item_id;
 const digitalRow=preparation.locator(`[data-item-id="${digital}"]`);
 await expect(digitalRow).toContainText('3 tracks · 0 ready');

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
 await imports.getByRole('button',{name:'Back to imports',exact:true}).click();
 const override=await reviewImport(imports,[cue,bin]);
 await expect(override.locator('option[value=default]')).toContainText('Opus 192 kbps');
 await override.selectOption('none');
 const cdIntake=await startImport(intakePage,imports);
 await expect.poll(async()=> outcome(intakePage,cdIntake),{timeout:45000}).toBe('succeeded');
 const cdImport=await member(intakePage,cdIntake);
 assert.equal(cdImport.preparation_profile,null);
 const cdRow=preparation.locator(`[data-item-id="${cdImport.target_item_id}"]`);
 await expect(cdRow).toContainText('1 tracks · 0 ready');

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
 await preparation.getByRole('button',{name:'Select all matching results'}).click();
 await preparation.getByRole('button',{name:'Preview Opus 128 kbps preparation'}).click();
 const batch=preparation.getByRole('region',{name:'Review batch'});
 await expect(batch.getByText('4 selected tracks · 0 already ready')).toBeVisible();
 await writeFile(refuse,'refuse');
 await batch.getByRole('button',{name:'Start Opus 128 kbps preparation'}).click();
 await expect(batch.getByText(/0 successful · 2 need attention · 0 pending/)).toBeVisible({timeout:45000});
 const failedBatch=(await request(page,`/items/${digital}/music-preparation-batches`)).find(batch=>batch.profile.bitrate_kbps===128);
 assert.ok(failedBatch.members.every(track=>/opusenc conversion failed: fixture encoder refusal/.test(track.error)));
 await unlink(refuse);
 await batch.getByRole('button',{name:'Retry unfinished tasks'}).click();
 await expect(batch.getByText(/2 successful · 0 need attention · 0 pending/)).toBeVisible({timeout:45000});
 await preparation.getByRole('button',{name:'Back to preparation results'}).click();
 await expect(preparation.getByText('No albums match these preparation filters.')).toBeVisible();

 // A different preset stays independently eligible in the single-album workflow.
 await bitrate.selectOption('192');
 await digitalRow.click();
 const album=preparation.getByRole('region',{name:'Album preparation'});
 await expect(album.getByRole('heading',{name:'Prepare album as Opus 192 kbps'})).toBeVisible();
 await unlink(gate);
 await album.getByRole('button',{name:'Prepare missing Opus 192 kbps tracks'}).click();
 await album.getByRole('button',{name:'Cancel outstanding tracks'}).click();
 await expect(album.getByText(/batch cancelled/)).toBeVisible();
 await writeFile(gate,'ready');
 await album.getByRole('button',{name:'Retry unfinished tracks'}).click();
 await expect(album.getByText('All album tracks are ready as Opus 192 kbps.',{exact:false})).toBeVisible({timeout:45000});
 const opusLink=album.getByRole('link',{name:/Opus 192 kbps/}).first();
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
 assert.match(failed.steps.find(step=>step.step==='preparation').message,/no supported audio tracks/);
 const retained=await request(intakePage,`/items/${brokenSummary.target_item_id}`);
 assert.equal(retained.captures[0].representations.length,1);
 assert.equal(retained.captures[0].representations[0].files[0].digest.hex,createHash('sha256').update(brokenBytes).digest('hex'));

 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 assert.deepEqual(errors,[]);
 console.log('Opus UI passed: persisted digital default, override, active import, exact originals, live cached list, bitrate filters, two-album batch and downloadable single-album output.');
}catch(error){
 console.error(logs);
 if(page)console.error(await page.locator('body').innerText({timeout:2000}).catch(()=>''));
 throw error;
}finally{
 await browser?.close();
 await stopProcess(server);
 await rm(scratch,{recursive:true,force:true});
}
