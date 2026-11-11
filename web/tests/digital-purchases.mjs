import {stopProcess} from './support/runtime.mjs';
// Actual browser setup, purchased-file intake and listening against an isolated server.
import {chromium,expect,chooseFolder} from './support/browser.mjs';
import {spawn,spawnSync,waitForServer,fixturePort} from './support/runtime.mjs';
import {mkdtemp,mkdir,writeFile,copyFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes,createHash,X509Certificate} from 'node:crypto';
import {clickAction} from './actions.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-purchases-'));
const config=path.join(scratch,'collector.toml');
const flac=process.env.COLLECTOR_FLAC||spawnSync('which',['flac'],{encoding:'utf8'}).stdout.trim();
assert.equal(spawnSync(flac,['--version']).status,0,'WAV acceptance requires COLLECTOR_FLAC or a real flac executable');
const password=randomBytes(24).toString('hex');
const publicPort=await fixturePort(),privatePort=await fixturePort(),lanPort=await fixturePort();
const origin=`https://localhost:${lanPort}`;
const fixture=path.join(scratch,'Digital purchase');await mkdir(fixture);
for(const file of ['01.flac','02.mp3'])await copyFile(path.join(root,'crates/collector-store/tests/fixtures/digital-audio',file),path.join(fixture,file));
await writeFile(path.join(fixture,'receipt.txt'),'PRIVATE ORDER REFERENCE 482');
await writeFile(path.join(fixture,'booklet.txt'),'Shared booklet');
await writeFile(path.join(fixture,'cover.png'),Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64'));
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:${publicPort}"\nprivate_bind = "127.0.0.1:${privatePort}"\nlan_bind = "127.0.0.1:${lanPort}"\nlocal_playback_bind = "127.0.0.1:0"\nauthentication_origin = "${origin}"\nallowed_origins = ["${origin}"]\ntls_names = ["localhost", "127.0.0.1"]\nendpoints = ["${origin}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n[inboxes]\npurchases = "${scratch}"\n[encoders]\nflac = "${flac}"\n`);
const binary=(process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server'));
const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let logs='';const server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
async function openFileTree(row){
 await row.locator('summary').filter({hasText:'Files and album playback'}).click();
 await row.locator('.import-file-tree').waitFor();
 for(;;){const closed=row.locator('.import-file-tree button[aria-expanded="false"]');if(!await closed.count())break;await closed.first().click();}
}
let browser,page;
try{
 await waitForServer(`http://127.0.0.1:${publicPort}/healthz`,server,{logs:()=>logs});
 // Trust only this disposable fixture's SPKI, including its service-worker fetch.
 const spki=createHash('sha256').update(new X509Certificate(await readFile(path.join(scratch,'state/tls/cert.pem'))).publicKey.export({type:'spki',format:'der'})).digest('base64');
 browser=await chromium.launch({mode:process.env.COLLECTOR_TEST_BROWSER||'chrome',headless:true,args:[`--ignore-certificate-errors-spki-list=${spki}`]});
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);
 page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(origin);await page.getByLabel('Username',{exact:true}).fill('owner');await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Digital shelf');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>{const response=await fetch('/api/v1/collection-search?limit=50');const data=await response.json();return data.items.find(c=>c.name==='Digital shelf').id;});
 await page.goto(`${origin}/imports?collection=${collection}`);
 const imports=page.getByRole('region',{name:'Imports workspace'});
 await imports.getByLabel("Choose one package's files").setInputFiles(['01.flac','02.mp3','receipt.txt','booklet.txt','cover.png'].map(file=>path.join(fixture,file)));
 await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
 await imports.getByLabel('Store',{exact:true}).fill('Private store');await imports.getByLabel('Purchase date').fill('2026-10-19');await imports.getByLabel('Order reference').fill('Private order 482');await imports.getByLabel('Price',{exact:true}).fill('1500');
 await openFileTree(imports);
 await imports.getByLabel('Use folder for album playback: All files',{exact:true}).check();
 await expect(imports.getByLabel(/Curators only: receipt.txt/)).toBeChecked();await expect(imports.getByLabel(/Curators only: booklet.txt/)).toBeChecked();await imports.getByLabel(/Curators only: booklet.txt/).uncheck();
 let releaseUpload;const held=new Promise(resolve=>{releaseUpload=resolve;});
 await page.route('**/api/v1/imports/*/files/*',async route=>{await held;await route.abort('aborted').catch(()=>{});},{times:1});
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));
 const intake=new URL(page.url()).searchParams.get('item');assert.ok(intake);
 await imports.getByRole('progressbar',{name:'Uploading files',exact:true}).waitFor();
 releaseUpload();await page.reload({waitUntil:'domcontentloaded'});
 await imports.getByLabel('Reselect package files to resume',{exact:true}).setInputFiles(['01.flac','02.mp3','receipt.txt','booklet.txt','cover.png'].map(file=>path.join(fixture,file)));

 let item;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);item=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:45000}).toBe('succeeded');
 const stored=await page.evaluate(async id=>await(await fetch(`/api/v1/items/${id}`)).json(),item);assert.equal(stored.title,'Purchased album');assert.equal(stored.specimens.length,0);assert.equal(stored.captures[0].purchase.store,'Private store');assert.equal(stored.captures[0].purchase.cost.minor_units,1500);assert.ok(stored.primary_artwork);
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);
 const tracks=page.getByRole('region',{name:'Album tracks'});await tracks.getByText('1. FLAC original',{exact:true}).waitFor();await tracks.getByText('2. MP3 original',{exact:true}).waitFor();assert.equal(await tracks.getByText('Inspect CD structure',{exact:true}).count(),0);
 const player=page.getByRole('region',{name:'Music player'});
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02;}return false;});
 await expect(player.getByRole('button',{name:'Queue (2)',exact:true})).toBeVisible();
 await player.getByRole('button',{name:'Next track',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentSrc.endsWith('/1');});
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await tracks.locator('.music-track').last().getByRole('button',{name:/^Play track /}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02&&audio.currentSrc.endsWith('/1');}return false;});
 await expect(player.getByRole('button',{name:'Queue (1)',exact:true})).toBeVisible();await expect(player.getByRole('button',{name:'Next track',exact:true})).toBeDisabled();
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 // A lossy original can supply the live encoder without saved derivatives.
 await player.getByLabel('Playback format').selectOption('opus');
 const liveMp3=page.waitForResponse(r=>r.request().method()==='GET'&&r.url().includes('/music-stream/')&&r.url().includes('format=opus'));
 await tracks.getByRole('button',{name:'Play track 2: MP3 original',exact:true}).click();
 assert.equal((await liveMp3).status(),200);
 await expect.poll(()=>player.locator('audio').evaluate(audio=>!audio.paused&&audio.currentTime>.02)).toBe(true);
 await player.getByRole('button',{name:'Pause',exact:true}).click();
 await player.getByLabel('Playback format').selectOption('saved');
 // Close an album while its real server response is withheld. A later response
 // cannot replace a new track selected after reopening the panel.
 let releaseAlbum;const heldAlbum=new Promise(resolve=>{releaseAlbum=resolve;});
 let albumRequestError;let albumRequested;const requestedAlbum=new Promise(resolve=>{albumRequested=resolve;});
 await page.route(`**/api/v1/items/${item}/client-playback`,async route=>{try{const response=await route.fetch();albumRequested();await heldAlbum;await route.fulfill({response}).catch(()=>{});}catch(error){albumRequestError=error;albumRequested();await route.abort().catch(()=>{});}},{times:1});
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();await requestedAlbum;assert.ifError(albumRequestError);
 await page.getByRole('button',{name:'Close details',exact:true}).click();
 await page.locator('.collection-row').filter({hasText:'Purchased album'}).click();
 await tracks.locator('.music-track').last().getByRole('button',{name:/^Play track /}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02&&audio.currentSrc.endsWith('/1');}return false;});
 releaseAlbum();await expect(player.getByRole('button',{name:'Queue (1)',exact:true})).toBeVisible();
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await clickAction(tracks,'Add album to list');const chooser=page.getByRole('dialog',{name:'Add to list',exact:true});await chooser.getByLabel('New list name').fill('Purchased playlist');await chooser.getByRole('button',{name:'Append to list',exact:true}).click();await chooser.getByRole('button',{name:'Open list',exact:true}).click();
 const lists=page.getByRole('region',{name:'Collection lists'});await expect(lists.locator('.saved-list-entries li')).toHaveCount(2);await lists.getByRole('button',{name:'Play playlist',exact:true}).click();
 await page.waitForFunction(()=>{const a=document.querySelector('.music-player audio');if(a){a.loop=true;return !a.paused&&a.currentTime>.02;}return false;});
 await page.waitForFunction(()=>navigator.mediaSession.metadata?.artist==='Digital artist'&&navigator.mediaSession.metadata.album==='Purchased album');
 await page.locator('.music-player audio').evaluate(a=>{a.loop=false;});
 await page.getByRole('button',{name:'Next track',exact:true}).click();
 await page.waitForFunction(()=>{const a=document.querySelector('.music-player audio');if(a&&a.currentSrc.endsWith('/1')){a.loop=true;return !a.paused&&a.currentTime>.02;}return false;});
 await page.locator('.music-player audio').evaluate(a=>{a.pause();a.currentTime=.1;});
 await page.reload();assert.equal(await page.locator('.music-player audio').evaluate(a=>a.paused),true);
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);await page.getByRole('button',{name:'Album management',exact:true}).click();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();
 const editor=page.getByRole('dialog',{name:'Edit entry'});await editor.getByRole('button',{name:'Music',exact:true}).click();await clickAction(editor,'Edit track details',0);
 await editor.getByLabel('Track title',{exact:true}).fill('My retained title');await editor.getByRole('button',{name:'Save track details',exact:true}).click();await editor.getByText('1. My retained title',{exact:true}).waitFor();
 await editor.getByText('Purchase details (private)',{exact:true}).click();await editor.getByRole('button',{name:'Edit purchase details',exact:true}).click();await editor.getByLabel('Store',{exact:true}).fill('Corrected private store');await editor.getByRole('button',{name:'Save purchase details',exact:true}).click();await editor.getByText('Corrected private store',{exact:true}).waitFor();
 await editor.getByRole('button',{name:'Close curator'}).click();await page.reload();await page.getByText('1. My retained title',{exact:true}).waitFor();
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.screenshot({path:path.join(scratch,'digital-mobile.png'),fullPage:true});
 await page.setViewportSize({width:1440,height:1000});
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
 await imports.getByRole('region',{name:'Inbox package chooser'}).getByLabel('Digital purchase',{exact:true}).check();
 await imports.getByRole('button',{name:'Review packages',exact:true}).click();
 await imports.getByLabel('Store',{exact:true}).fill('Separate inbox purchase');
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const inboxId=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,inboxId),{timeout:60000}).toBe('completed');
 const inboxItem=await page.evaluate(async id=>{const data=await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json();return data.items[0].import.target_item_id;},inboxId);
 assert.notEqual(inboxItem,item);
 const inboxCapture=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),inboxItem);
 assert.equal(inboxCapture.specimens.length,0);assert.equal(inboxCapture.captures[0].association.specimen_id,null);assert.equal(inboxCapture.captures[0].purchase.store,'Separate inbox purchase');
 await writeFile(path.join(fixture,'broken.mp3'),Buffer.from('not a valid audio stream'));
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(path.join(fixture,'broken.mp3'));
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const brokenId=new URL(page.url()).searchParams.get('item');let broken;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,brokenId);broken=members[0]?.import?.target_item_id;return members[0]?.import?.phase==='published'?members[0].import.outcome:'waiting_for_preservation';},{timeout:60000}).toBe('failed');
 await page.goto(`${origin}/?collection=${collection}&item=${broken}`);
 await page.getByText(/playable audio analysis is not complete/).waitFor();
 assert.equal(await page.getByText('Inspect CD structure',{exact:true}).count(),0);
 // Same-format copies still require a choice after tag analysis.
 await copyFile(path.join(fixture,'01.flac'),path.join(fixture,'alternate.flac'));
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(['01.flac','alternate.flac'].map(name=>path.join(fixture,name)));
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const alternateIntake=new URL(page.url()).searchParams.get('item');let alternate;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,alternateIntake);alternate=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:60000}).toBe('succeeded');
 await page.goto(`${origin}/?collection=${collection}&item=${alternate}`);
 await tracks.getByText(/Alternate encodings need a choice/).waitFor();
 await expect(page.locator('.album-playback-notice')).toContainText('Choose which alternate encodings');
 const attentionBefore=await page.evaluate(async collection=>(await(await fetch(`/api/v1/items?collection_id=${collection}&filter=album_playback_needs_attention`)).json()).items.map(item=>item.id),collection);
 assert.ok(attentionBefore.includes(alternate),'ambiguous purchase missing from playback attention results');

 const beforeAmbiguous=await page.evaluate(()=>Object.entries(localStorage).filter(([key])=>key.startsWith('collector.music-queue.v1.')));
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await tracks.getByRole('alert').getByText(/Choose which alternate encodings/).waitFor();
 assert.deepEqual(await page.evaluate(()=>Object.entries(localStorage).filter(([key])=>key.startsWith('collector.music-queue.v1.'))),beforeAmbiguous);
 await tracks.locator('.music-track').last().getByRole('button',{name:/^Play track /}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02&&audio.currentSrc.endsWith('/1');}return false;});
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await page.getByRole('button',{name:'Album management',exact:true}).click();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();
 await editor.getByRole('button',{name:'Music',exact:true}).click();
 for(const [index,included] of [[0,true],[1,false]]){
  await clickAction(editor,'Edit track details',index);
  await editor.getByLabel('Include in album playback and lists',{exact:true}).setChecked(included);
  await editor.getByRole('button',{name:'Save track details',exact:true}).click();
  await expect(editor.getByRole('form',{name:'Edit purchased track'})).toHaveCount(0);
 }
 await editor.getByRole('button',{name:'Close curator'}).click();
 await expect(page.locator('.album-playback-notice')).toHaveCount(0);
 const attentionAfter=await page.evaluate(async collection=>(await(await fetch(`/api/v1/items?collection_id=${collection}&filter=album_playback_needs_attention`)).json()).items.map(item=>item.id),collection);
 assert.ok(!attentionAfter.includes(alternate),'repaired purchase remains in playback attention results');
 await page.reload();
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02&&audio.currentSrc.endsWith('/0');}return false;});
 await expect(player.getByRole('button',{name:'Queue (1)',exact:true})).toBeVisible();
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 // WAV is a digital original; the normal intake option prepares native-format FLAC.
 const wavFile=path.join(scratch,'03.wav');
 const cleanWav=await readFile(path.join(root,'crates/collector-store/tests/fixtures/digital-audio/03.wav'));
 const suffixedWav=Buffer.concat([cleanWav,Buffer.from('ID3\xffuninterpreted trailer\xff','latin1')]);
 await writeFile(wavFile,suffixedWav);
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(wavFile);
 await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
 await imports.getByLabel('Store',{exact:true}).fill('WAV store');
 await imports.getByLabel('Audio preparation after import',{exact:true}).selectOption('flac');
 await expect(imports.getByLabel('Physical copy label')).toHaveCount(0);
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));
 const wavIntake=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,wavIntake),{timeout:60000}).toBe('completed');
 await imports.getByRole('button',{name:'Review import',exact:true}).click();
 const wavReview=page.getByRole('dialog',{name:'Import review',exact:true});
 await expect(wavReview).toContainText('trailing bytes outside their declared RIFF segment');
 await page.reload();
 await imports.getByRole('button',{name:'Review import',exact:true}).click();
 await expect(wavReview).toContainText('trailing bytes outside their declared RIFF segment');
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.screenshot({path:path.join(scratch,'wav-trailing-warning-mobile.png'),fullPage:true});
 await wavReview.getByRole('button',{name:'Close import review',exact:true}).click();
 await page.setViewportSize({width:1440,height:1000});
 const wavItem=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items[0].import.target_item_id,wavIntake);
 const wavStored=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),wavItem);
 assert.equal(wavStored.specimens.length,0);assert.equal(wavStored.captures[0].purchase.store,'WAV store');
 assert.equal(wavStored.captures[0].representations.length,2);
 const wavSource=wavStored.captures[0].representations.find(rep=>rep.files.some(file=>file.name==='03.wav'));
 const originalDigest=await page.evaluate(async ({item,source})=>{
  const response=await fetch(`/api/v1/items/${item}/representations/${source}/0`);if(!response.ok)throw Error('Original WAV download failed');
  const bytes=await response.arrayBuffer();return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),b=>b.toString(16).padStart(2,'0')).join('');
 },{item:wavItem,source:wavSource.id});
 assert.equal(originalDigest,createHash('sha256').update(suffixedWav).digest('hex'));
 const wavRows=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/music-tracks`)).json()).filter(row=>row.profile?.codec==='flac'),wavItem);
 assert.equal(wavRows.length,1);assert.equal(wavRows[0].audio_format.sample_rate,96000);
 assert.equal(wavRows[0].audio_format.bits_per_sample,24);assert.equal(wavRows[0].audio_format.channels,1);
 await page.goto(`${origin}/?collection=${collection}&item=${wavItem}`);
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02;}return false;});
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await page.reload();await tracks.getByText('1. 03.wav',{exact:true}).waitFor();
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.screenshot({path:path.join(scratch,'wav-mobile.png'),fullPage:true});
 // Preparation remains available later from the real Music editor.
 await page.setViewportSize({width:1440,height:1000});
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(wavFile);
 await imports.getByLabel('Audio preparation after import',{exact:true}).selectOption('none');
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));
 const laterIntake=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,laterIntake),{timeout:60000}).toBe('completed');
 const laterItem=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items[0].import.target_item_id,laterIntake);
 await page.goto(`${origin}/?collection=${collection}&item=${laterItem}`);
 // The strict WAV reader plays the declared 24-bit segment before any FLAC is prepared.
 await player.getByLabel('Playback format').selectOption('flac');
 const liveWav=page.waitForResponse(r=>r.request().method()==='GET'&&r.url().includes('/music-stream/')&&r.url().includes('format=flac'));
 await tracks.getByRole('button',{name:'Play track 1: 03.wav',exact:true}).click();
 const converted=await liveWav;
 assert.equal(converted.status(),200);
 assert.equal(converted.headers()['accept-ranges'],'none');
 await expect.poll(()=>player.locator('audio').evaluate(audio=>!audio.paused&&audio.currentTime>.02)).toBe(true);
 await player.getByRole('button',{name:'Pause',exact:true}).click();
 await player.getByLabel('Playback format').selectOption('saved');
 await page.getByRole('button',{name:'Album management',exact:true}).click();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();
 await editor.getByRole('button',{name:'Music',exact:true}).click();
 const prepareLater=editor.getByRole('region',{name:'Prepare album',exact:true});
 await expect(prepareLater.getByLabel('Include this original',{exact:true})).toBeChecked();
 await prepareLater.getByRole('button',{name:'Prepare missing FLAC tracks',exact:true}).click();
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/music-tracks`)).json())[0]?.state.status,laterItem),{timeout:45000}).toBe('available');
 // Cached track availability may need two five-second refresh cycles after
 // the worker publishes; backend availability alone does not refresh the UI.
 const preparedPlay=editor.getByRole('button',{name:'Play track 1: 03.wav',exact:true});
 await expect(preparedPlay).toBeEnabled({timeout:15000});
 await preparedPlay.click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02;}return false;});
 await editor.getByRole('button',{name:'Close curator'}).click();
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await page.reload();await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02;}return false;});
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 // Trailing INFO/ID3 supplies real metadata/artwork; downloads are portable tagged FLACs.
 const taggedFile=path.join(root,'crates/collector-store/tests/fixtures/digital-audio/03-tagged.wav');
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(taggedFile);
 await imports.getByLabel('Audio preparation after import',{exact:true}).selectOption('flac');
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const taggedIntake=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,taggedIntake),{timeout:60000}).toBe('completed');
 const taggedItem=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items[0].import.target_item_id,taggedIntake);
 await page.goto(`${origin}/?collection=${collection}&item=${taggedItem}`);
 await tracks.getByText('7. WAV title',{exact:true}).waitFor();
 await expect(page.getByRole('region',{name:'Selected entry'}).getByRole('img').first()).toBeVisible();
 const downloadEvent=page.waitForEvent('download');await tracks.getByRole('link',{name:/Download FLAC/}).click();
 const download=await downloadEvent;assert.ok(download.suggestedFilename().includes('Tagged WAV album'));assert.ok(download.suggestedFilename().endsWith('.flac'));
 const exportPath=path.join(scratch,'tagged-export.flac');await download.saveAs(exportPath);
 const metadataTool=process.env.COLLECTOR_METAFLAC||path.join(path.dirname(flac),'metaflac');
 const exportedTags=spawnSync(metadataTool,['--export-tags-to=-',exportPath],{encoding:'utf8'});assert.equal(exportedTags.status,0,exportedTags.stderr);
 assert.match(exportedTags.stdout,/TITLE=WAV title/);assert.match(exportedTags.stdout,/ALBUM=Tagged WAV album/);assert.match(exportedTags.stdout,/ARTIST=Tagged WAV artist/);assert.match(exportedTags.stdout,/TRACKNUMBER=7/);assert.match(exportedTags.stdout,/DISCNUMBER=2/);
 const picturePath=path.join(scratch,'export-cover.png');assert.equal(spawnSync(metadataTool,[`--export-picture-to=${picturePath}`,exportPath]).status,0);
 assert.ok((await readFile(picturePath)).length>0);
 // Refresh the existing retained source without another upload or replacing the capture.
 const beforeAnalysis=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),taggedItem);
 await page.getByRole('button',{name:'Album management',exact:true}).click();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();await editor.getByRole('button',{name:'Music',exact:true}).click();
 await editor.getByRole('button',{name:/Analyze retained digital audio/}).click();
 const analysis=editor.getByRole('region',{name:'Digital audio analysis',exact:true});
 const run=analysis.getByRole('button',{name:'Analyze retained audio',exact:true});
 await expect(run).toBeEnabled();
 const analysisPost=page.waitForResponse(response=>response.url().endsWith('/operations')&&response.request().method()==='POST').then(async response=>({ok:response.ok(),body:await response.json()})).catch(error=>({ok:false,error:String(error)}));
 await run.click();
 const analysisResponse=await analysisPost;assert.ok(analysisResponse.ok,JSON.stringify(analysisResponse));const analysisReceipt=analysisResponse.body;assert.ok(analysisReceipt.job_id);
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/jobs/${id}`)).json()).state,analysisReceipt.job_id),{timeout:45000}).toBe('completed');
 await expect(analysis.getByText(/Digital audio analyzed · 1 tracks/).first()).toBeVisible({timeout:45000});
 await editor.getByRole('button',{name:'Music',exact:true}).click();await editor.getByText('7. WAV title',{exact:true}).waitFor();
 await editor.getByRole('button',{name:'Close curator'}).click();
 const afterAnalysis=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),taggedItem);
 assert.deepEqual(afterAnalysis.captures,beforeAnalysis.captures);assert.equal(afterAnalysis.primary_artwork,beforeAnalysis.primary_artwork);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.screenshot({path:path.join(scratch,'wav-metadata-mobile.png'),fullPage:true});
 // Keep this independent import journey on a fresh page in the same signed-in
 // context. Hosted Chrome can stop delivering directory input events on the
 // previous long-lived export/playback page, including for a plain file input.
 await page.close();page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 const steamImports=page.getByRole('region',{name:'Imports workspace'});
 // Folder review uses names only and bounds rendered children even for large inventories.
 const large=path.join(scratch,'Large review');await mkdir(large);
 for(let i=0;i<1001;i++)await writeFile(path.join(large,`${String(i).padStart(4,'0')}.flac`),'review inventory only');
 await page.goto(`${origin}/imports?collection=${collection}`);
 await chooseFolder(page,steamImports.getByLabel('Choose folders',{exact:true}),large);
 await steamImports.locator('summary').filter({hasText:'Files and album playback'}).click();
 assert.equal(await steamImports.locator('.import-tree-file').count(),0);
 await steamImports.getByRole('button',{name:'▸ All files',exact:true}).click();
 await steamImports.getByRole('button',{name:'▸ Large review',exact:true}).click();
 const fileLabels=()=>steamImports.locator('.import-tree-file input[aria-label^="Use for album playback:"]').evaluateAll(inputs=>inputs.map(input=>input.getAttribute('aria-label')));
 const firstPage=await fileLabels();assert.equal(firstPage.length,50);
 await steamImports.getByRole('button',{name:'Next entries',exact:true}).click();
 const secondPage=await fileLabels();assert.equal(secondPage.length,50);
 assert.equal(new Set([...firstPage,...secondPage]).size,100);
 // Steam-style format wrappers: real folder discovery, review and publication.
 const steam=path.join(scratch,'Steam soundtracks');
 for(const folder of ['Game Name OST/FLAC/Disc 1','Game Name OST/MP3/Disc 1','Game Name OST/WAV/Disc 1','Game Name OST/Booklet','Other Album/WAV','Wild Album/All FLAC','Wild Album/DLC MP3'])await mkdir(path.join(steam,folder),{recursive:true});
 await copyFile(path.join(fixture,'01.flac'),path.join(steam,'Game Name OST/FLAC/Disc 1/01.flac'));
 await copyFile(path.join(fixture,'02.mp3'),path.join(steam,'Game Name OST/MP3/Disc 1/01.mp3'));
 await copyFile(wavFile,path.join(steam,'Other Album/WAV/01.wav'));
 await copyFile(wavFile,path.join(steam,'Game Name OST/WAV/Disc 1/01.wav'));
 for(const name of ['01.flac','02.flac'])await copyFile(path.join(fixture,'01.flac'),path.join(steam,'Wild Album/All FLAC',name));
 await copyFile(path.join(fixture,'02.mp3'),path.join(steam,'Wild Album/DLC MP3/01.mp3'));
 await copyFile(path.join(fixture,'cover.png'),path.join(steam,'Game Name OST/cover.png'));
 await writeFile(path.join(steam,'Game Name OST/Booklet/booklet.txt'),'PRIVATE STEAM BOOKLET');
 await page.goto(`${origin}/imports?collection=${collection}`);
 await chooseFolder(page,steamImports.getByLabel('Choose folders',{exact:true}),steam);
 await steamImports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
 const packageRows=steamImports.locator('.intake-review-row');await expect(packageRows).toHaveCount(3);
 const flacRow=packageRows.filter({has:page.getByText('Game Name OST',{exact:true})});
 const wildRow=packageRows.filter({has:page.getByText('Wild Album',{exact:true})});
 const otherRow=packageRows.filter({has:page.getByText('Other Album',{exact:true})});
 await expect(flacRow).toHaveCount(1);await expect(wildRow).toHaveCount(1);await expect(otherRow).toHaveCount(1);
 await expect(steamImports.getByRole('button',{name:'Start import',exact:true})).toBeDisabled();
 await expect(wildRow).toContainText('Needs attention');
 await openFileTree(wildRow);
 await wildRow.getByRole('button',{name:'Select FLAC',exact:true}).click();
 await wildRow.getByLabel(/Use for album playback: .*All FLAC\/02.flac$/).uncheck();
 await wildRow.getByLabel(/Disc for .*All FLAC\/01.flac$/).selectOption('2');
 // A saved draft restores choices independently of the browser's file handles.
 await expect(steamImports.getByRole('status').filter({hasText:'Import review saved'})).toBeVisible();
 await page.reload();await expect(packageRows).toHaveCount(3);
 await openFileTree(wildRow);
 await expect(wildRow.getByLabel(/Use for album playback: .*All FLAC\/02.flac$/)).not.toBeChecked();
 await expect(wildRow.getByLabel(/Disc for .*All FLAC\/01.flac$/)).toHaveValue('2');
 await chooseFolder(page,steamImports.getByLabel('Reselect folder for reviewed import',{exact:true}),steam);
 await openFileTree(flacRow);
 await expect(flacRow.getByLabel(/Curators only: .*Booklet\/booklet.txt/)).toBeChecked();
 await expect(flacRow.getByLabel(/Curators only: .*cover.png/)).not.toBeChecked();
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.screenshot({path:path.join(scratch,'format-discovery-mobile.png'),fullPage:true});await page.setViewportSize({width:1440,height:1000});
 await steamImports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const steamIntake=new URL(page.url()).searchParams.get('item');
 let steamMembers;
 await expect.poll(async()=>{steamMembers=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,steamIntake);return steamMembers.filter(m=>m.import?.outcome==='succeeded').length;},{timeout:45000}).toBe(3);
 assert.equal(new Set(steamMembers.map(m=>m.import.target_item_id)).size,3);
 const flacMember=steamMembers.find(m=>m.plan.default_title==='Game Name OST');
 let steamAlbum;
 await expect.poll(async()=>{steamAlbum=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),flacMember.import.target_item_id);return !!steamAlbum.primary_artwork;},{timeout:45000}).toBe(true);
 assert.equal(steamAlbum.captures.length,1);assert.equal(steamAlbum.specimens.length,0);
 const steamFiles=steamAlbum.captures[0].representations[0].files;
 assert.equal(steamFiles.length,5);
 const selectedSteam=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks,flacMember.import.target_item_id);
 assert.equal(selectedSteam.length,1);assert.equal(selectedSteam[0].audio_format.codec,'flac');
 const wildMember=steamMembers.find(m=>m.plan.default_title==='Wild Album');
 const selectedWild=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks,wildMember.import.target_item_id);
 assert.equal(selectedWild.length,1);assert.equal(selectedWild[0].disc_position,2);
 assert.ok(steamFiles.some(file=>file.name.endsWith('/Booklet/booklet.txt')&&file.visibility==='curators'));
 assert.ok(steamFiles.some(file=>file.name.endsWith('/cover.png')&&(file.visibility??'shared')==='shared'));
 assert.ok(steamAlbum.primary_artwork,'cover enrichment completes for the imported package');
 // Use real cursor responses with small pages to exercise selection beyond the
 // rendered rows without importing dozens of extra audio packages.
 let releasePage;
 const heldPage=new Promise(resolve=>{releasePage=resolve;});
 let continuationRequests=0;
 const shortMembers=async route=>{
  const url=new URL(route.request().url());
  url.searchParams.set('limit','1');
  if(url.searchParams.get('cursor')==='0'&&continuationRequests++===0)await heldPage;
  await route.fulfill({response:await route.fetch({url:url.href})});
 };
 await page.route(`**/api/v1/intakes/${steamIntake}/members?*`,shortMembers);
 await page.reload();
 const published=steamImports.locator('.intake-members .care-row');
 const morePackages=steamImports.getByRole('button',{name:'More packages',exact:true});
 await expect(published).toHaveCount(1);
 await morePackages.click();
 try{
  const loading=steamImports.getByRole('button',{name:'Loading packages…',exact:true});
  await expect(loading).toBeDisabled();
  await expect(loading.locator('.page-spinner')).toBeVisible();
  await expect.poll(()=>continuationRequests).toBe(1);
 }finally{releasePage();}
 await expect(published).toHaveCount(2);
 const backgrounds=await published.evaluateAll(rows=>rows.map(row=>getComputedStyle(row).backgroundColor));
 assert.notEqual(backgrounds[0],backgrounds[1],'Published rows should have alternating backgrounds');

 const readMetadata=id=>page.evaluate(async id=>{
  const response=await fetch(`/api/v1/items/${id}/metadata`);
  if(!response.ok)throw Error(await response.text());
  return response.json();
 },id);
 const initial=await Promise.all(steamMembers.map(member=>readMetadata(member.import.target_item_id)));
 const remaining=steamMembers[2].import.target_item_id;
 const putPath=`**/api/v1/items/${remaining}/metadata`;
 const rejectOnce=async route=>{
  if(route.request().method()!=='PUT')return route.fallback();
  await page.unroute(putPath,rejectOnce);
  await route.fulfill({status:409,json:{error:{code:'revision_conflict',message:'Album changed while saving'}}});
 };
 await page.route(putPath,rejectOnce);
 await page.setViewportSize({width:390,height:844});
 await steamImports.getByRole('checkbox',{name:'Select all imported albums'}).check();
 await published.first().getByRole('checkbox').uncheck();
 const editSelected=steamImports.getByRole('button',{name:'Edit selected',exact:true});
 await editSelected.click();
 const albumEditor=page.getByRole('dialog',{name:'Edit selected albums'});
 const saveAlbums=albumEditor.getByRole('button',{name:'Save album changes'});
 await expect(albumEditor).toContainText('overwrite for 2 albums');
 await expect(saveAlbums).toBeDisabled();
 await albumEditor.getByRole('checkbox',{name:'Overwrite digital source'}).check();
 await albumEditor.getByRole('textbox',{name:'Digital source'}).fill('Steam');
 await albumEditor.getByRole('checkbox',{name:'Overwrite album artist'}).check();
 await albumEditor.getByRole('textbox',{name:'Album artist'}).fill('Steam soundtrack artists');
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
 await page.screenshot({path:path.join(scratch,'bulk-album-editor-mobile.png')});
 await saveAlbums.click();
 await expect(albumEditor.getByRole('status')).toContainText('1 album saved.');
 await expect(albumEditor.getByRole('alert')).toContainText('Album changed while saving');
 const updated=await readMetadata(steamMembers[1].import.target_item_id);
 assert.equal(updated.display.domain_fields.digital_source.value,'Steam');
 assert.equal(updated.display.subtitle.value,'Steam soundtrack artists');
 assert.equal((await readMetadata(remaining)).revision,initial[2].revision);
 await albumEditor.getByRole('button',{name:'Retry remaining albums'}).click();
 await expect(albumEditor.getByRole('status')).toContainText('2 albums saved.');
 await albumEditor.getByRole('button',{name:'Done',exact:true}).click();
 await expect(editSelected).toBeFocused();
 const final=await Promise.all(steamMembers.map(member=>readMetadata(member.import.target_item_id)));
 assert.equal(final[0].revision,initial[0].revision,'Excluded album was changed');
 assert.equal(final[1].revision,updated.revision,'Retry reapplied a saved album');
 assert.equal(final[2].display.domain_fields.digital_source.value,'Steam','Selection missed an unloaded page');
 for(let index=1;index<3;index++){
  assert.equal(final[index].display.title.value,initial[index].display.title.value,'Unchecked title was changed');
  assert.equal(final[index].display.artwork,initial[index].display.artwork,'Artwork changed');
 }
 await page.unroute(`**/api/v1/intakes/${steamIntake}/members?*`,shortMembers);
 await page.setViewportSize({width:1440,height:1000});
 let releaseSaved;
 const heldSaved=new Promise(resolve=>{releaseSaved=resolve;});
 const shortSaved=async route=>{
  const url=new URL(route.request().url());
  url.searchParams.set('limit','2');
  if(url.searchParams.has('cursor'))await heldSaved;
  await route.fulfill({response:await route.fetch({url:url.href})});
 };
 await page.route('**/api/v1/intakes?*',shortSaved);
 await steamImports.getByRole('button',{name:'Back to imports',exact:true}).click();
 const savedImports=steamImports.getByRole('region',{name:'Saved imports'});
 await expect(savedImports.locator('button.collection-row').first()).toContainText('3 packages');
 const savedRows=savedImports.locator('button.collection-row');
 await expect(savedRows).toHaveCount(2);
 assert.notEqual(...await savedRows.evaluateAll(rows=>rows.map(row=>getComputedStyle(row).backgroundColor)));
 await savedImports.getByRole('button',{name:'More imports',exact:true}).click();
 try{
  const loading=savedImports.getByRole('button',{name:'Loading imports…',exact:true});
  await expect(loading).toBeDisabled();
  await expect(loading.locator('.page-spinner')).toBeVisible();
 }finally{releaseSaved();}
 await expect(savedRows).toHaveCount(4);
 await page.unroute('**/api/v1/intakes?*',shortSaved);
 // Immediate Library refresh exposes the newly imported version and its track.
 await page.goto(`${origin}/?collection=${collection}&item=${flacMember.import.target_item_id}`);
 await page.getByRole('region',{name:'Album tracks'}).getByText('1. FLAC original',{exact:true}).waitFor();
 // Separate top-level digital discs may join one reviewed album without
 // borrowing a physical carrier or a MusicBrainz medium assignment.
 await page.close();page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
 const discImports=page.getByRole('region',{name:'Imports workspace'});
 const discRows=discImports.locator('.intake-review-row');
 const discs=path.join(scratch,'Two discs');
 for(const disc of [1,2]){
  const folder=path.join(discs,`Disc ${disc}`);await mkdir(folder,{recursive:true});
  await copyFile(path.join(fixture,'02.mp3'),path.join(folder,'02.mp3'));
  await copyFile(wavFile,path.join(folder,'03.wav'));
 }
 await page.goto(`${origin}/imports?collection=${collection}`);
 await discImports.getByLabel('Folder layout',{exact:true}).selectOption('items');
 await chooseFolder(page,discImports.getByLabel('Choose folders',{exact:true}),discs);
 await expect(discRows).toHaveCount(2);
 for(const row of await discRows.all()){
  await row.getByLabel('Album group',{exact:true}).fill('Two reviewed discs');
  await row.locator('summary').filter({hasText:'Files and album playback'}).click();
  await row.getByRole('button',{name:'Select MP3',exact:true}).click();
 }
 await discImports.getByLabel('Audio preparation after import',{exact:true}).selectOption('flac');
 // A failed replacement preview must leave the previously reviewed files attached.
 await page.route('**/api/v1/ingests/plan',route=>route.fulfill({status:422,json:{error:{code:'invalid',message:'Replacement review rejected'}}}),{times:1});
 await discImports.getByLabel("Choose one package's files").setInputFiles(wavFile);
 await expect(discImports.getByRole('alert')).toContainText('Replacement review rejected');

 await discImports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const groupedIntake=new URL(page.url()).searchParams.get('item');let groupedMembers;
 await expect.poll(async()=>{groupedMembers=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,groupedIntake);return groupedMembers.filter(member=>member.import?.outcome==='succeeded').length;},{timeout:60000}).toBe(2);
 assert.equal(new Set(groupedMembers.map(member=>member.import.target_item_id)).size,1);
 const groupedPlayback=await page.evaluate(async id=>{const response=await fetch(`/api/v1/items/${id}/client-playback`);if(!response.ok)throw Error(await response.text());return await response.json();},groupedMembers[0].import.target_item_id);
 assert.deepEqual(groupedPlayback.tracks.map(track=>track.disc_position),[1,2]);
 assert.deepEqual(groupedPlayback.tracks.map(track=>track.audio_format.codec),['mp3','mp3']);
 const groupedAlbum=await page.evaluate(async id=>(await fetch(`/api/v1/items/${id}`)).json(),groupedMembers[0].import.target_item_id);
 assert.deepEqual(groupedAlbum.captures.map(capture=>capture.representations.length),[1,1],
  'No preparation batch should encode the other disc’s excluded WAV');
 assert.deepEqual(errors,[]);console.log(`Digital purchase browser acceptance passed (Chrome ${await browser.version()}): real setup, reviewed sharing/purchase, interrupted upload/resume, configured inbox, publication, artwork, direct album/track FLAC+MP3 Play, alternate-encoding review, obsolete-request fencing, playlists, paused reload, manual title/purchase edits, failed-source guidance, WAV digital intake/native FLAC preparation/real playback/reload, embedded tags/cover, portable FLAC metadata/artwork export, in-place source analysis, Steam format-folder discovery, paged bulk album metadata and failure recovery, striped rows, loading states and mobile layout. Artifacts: ${scratch}`);
}catch(error){console.error(error);console.error(logs);console.error('Browser state:',page?.url(),await page?.locator('body').innerText({timeout:2000}).catch(()=>''));await page?.screenshot({path:path.join(scratch,'failure.png'),fullPage:true}).catch(()=>{});throw error;}finally{try{await browser?.close();}finally{await stopProcess(server);}}
