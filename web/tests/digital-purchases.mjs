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
// Restored import review choices live in IndexedDB; read them as saved state.
async function intakeDrafts(){return page.evaluate(()=>new Promise((resolve,reject)=>{const request=indexedDB.open('collector-intake-drafts',1);request.onerror=()=>reject(request.error);request.onsuccess=()=>{const values=[];const cursor=request.result.transaction('drafts','readonly').objectStore('drafts').openCursor();cursor.onerror=()=>reject(cursor.error);cursor.onsuccess=()=>{const next=cursor.result;if(next){values.push(next.value);next.continue();}else resolve(values);};};}));}
async function expectQueue(player,count){
 await player.locator('.queue-toggle').click();
 await expect(page.getByRole('region',{name:'Currently playing list'}).locator('ol > li')).toHaveCount(count);
 await player.locator('.queue-toggle').click();
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
 await admin.getByLabel('Collection name').fill('Digital shelf');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await expect.poll(()=>page.evaluate(async()=>(await(await fetch('/api/v1/collection-search?limit=50')).json()).items.map(c=>c.name))).toContain('Digital shelf');await expect(admin.getByRole('alert')).toHaveCount(0);await admin.getByRole('button',{name:'Close administration'}).click();
 await page.getByRole('button',{name:'Administration',exact:true}).click();
 await admin.getByLabel('Collection name').fill('Game Purchases');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.games');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await expect.poll(()=>page.evaluate(async()=>(await(await fetch('/api/v1/collection-search?limit=50')).json()).items.map(c=>c.name))).toContain('Game Purchases');await expect(admin.getByRole('alert')).toHaveCount(0);await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>{const response=await fetch('/api/v1/collection-search?limit=50');const data=await response.json();return data.items.find(c=>c.name==='Digital shelf').id;});
 await page.goto(`${origin}/imports?collection=${collection}`);
 const imports=page.getByRole('region',{name:'Imports workspace'});
 await imports.getByLabel("Choose one package's files").setInputFiles(['01.flac','02.mp3','receipt.txt','booklet.txt','cover.png'].map(file=>path.join(fixture,file)));
 await expect(imports.getByRole('button',{name:'Start import',exact:true})).toBeVisible();
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
 await imports.getByLabel('Reselect files to resume upload',{exact:true}).setInputFiles(['01.flac','02.mp3','receipt.txt','booklet.txt','cover.png'].map(file=>path.join(fixture,file)));

 let item;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);item=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:45000}).toBe('succeeded');
 const stored=await page.evaluate(async id=>await(await fetch(`/api/v1/items/${id}`)).json(),item);assert.equal(stored.title,'Purchased album');assert.equal(stored.specimens.length,0);assert.equal(stored.captures[0].purchase.store,'Private store');assert.equal(stored.captures[0].purchase.cost.minor_units,1500);assert.ok(stored.primary_artwork);
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);
 const tracks=page.getByRole('region',{name:'Album tracks'});await expect(tracks.locator('.music-track[data-track-position="1"] .track-title strong')).toHaveAttribute('title','FLAC original');await expect(tracks.locator('.music-track[data-track-position="2"] .track-title strong')).toHaveAttribute('title','MP3 original');assert.equal(await tracks.getByRole('button',{name:'Inspect CD structure'}).count(),0);
 const player=page.getByRole('region',{name:'Music player'});
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02;}return false;});
 await expectQueue(player,2);
 await player.getByRole('button',{name:'Next track',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentSrc.endsWith('/1');});
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await tracks.locator('.music-track').last().getByRole('button',{name:/^Play track /}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02&&audio.currentSrc.endsWith('/1');}return false;});
 await expectQueue(player,1);await expect(player.getByRole('button',{name:'Next track',exact:true})).toBeDisabled();
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 // A lossy original can supply the live encoder without saved derivatives.
 await player.getByLabel('Playback format').selectOption('low_bandwidth');
 const liveMp3=page.waitForResponse(r=>r.request().method()==='GET'&&r.url().includes('/music-stream/')&&r.url().includes('mode=low_bandwidth'));
 await tracks.getByRole('button',{name:'Play track 2: MP3 original',exact:true}).click();
 assert.equal((await liveMp3).status(),200);
 await expect.poll(()=>player.locator('audio').evaluate(audio=>!audio.paused&&audio.currentTime>.02)).toBe(true);
 await player.getByRole('button',{name:'Pause',exact:true}).click();
 await player.getByLabel('Playback format').selectOption('high_bandwidth');
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
 releaseAlbum();await expectQueue(player,1);
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
 await editor.getByLabel('Track title',{exact:true}).fill('My retained title');await editor.getByRole('button',{name:'Save track details',exact:true}).click();await expect(editor.locator('.music-track[data-track-position="1"] .track-title strong')).toHaveAttribute('title','My retained title');
 await editor.locator('details').filter({has:page.getByRole('button',{name:'Edit purchase details',exact:true,includeHidden:true})}).locator('> summary').click();await editor.getByRole('button',{name:'Edit purchase details',exact:true}).click();await editor.getByLabel('Store',{exact:true}).fill('Corrected private store');await editor.getByRole('button',{name:'Save purchase details',exact:true}).click();await editor.getByText('Corrected private store',{exact:true}).waitFor();
 await editor.getByRole('button',{name:'Close curator'}).click();await page.reload();await expect(tracks.locator('.music-track[data-track-position="1"] .track-title strong')).toHaveAttribute('title','My retained title');
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.setViewportSize({width:1440,height:1000});
 // Mixed files in the same inbox directory, first reviewed under the wrong collection.
 await copyFile(path.join(fixture,'01.flac'),path.join(fixture,'02.FLAC'));
 await copyFile(path.join(fixture,'02.mp3'),path.join(fixture,'01.mp3'));
 const gameCollection=await page.evaluate(async()=>{const response=await fetch('/api/v1/collection-search?limit=50');return (await response.json()).items.find(c=>c.name==='Game Purchases').id;});
 async function reviewInboxPurchase(){
  await imports.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
  await imports.getByRole('region',{name:'Inbox package chooser'}).getByLabel('Digital purchase',{exact:true}).check();
  await imports.getByRole('button',{name:'Review packages',exact:true}).click();
 }
 await page.goto(`${origin}/imports?collection=${gameCollection}`);
 await reviewInboxPurchase();
 const destinationHint=imports.getByRole('region',{name:'Audio import destination'});
 await expect(destinationHint).toContainText('Game Purchases');
 await destinationHint.getByRole('button',{name:'Switch to Digital shelf'}).click();
 await page.waitForURL(url=>url.searchParams.get('collection')===collection);
 await reviewInboxPurchase();
 await openFileTree(imports);
 const playbackBox=name=>imports.getByRole('checkbox',{name:`Use for album playback: ${name}`,exact:true});
 const playbackRows=imports.getByRole('checkbox',{name:/^Use for album playback:/});
 assert.deepEqual(await playbackRows.evaluateAll(inputs=>inputs.map(input=>input.getAttribute('aria-label'))),
  ['01.flac','02.FLAC','01.mp3','02.mp3'].map(name=>`Use for album playback: ${name}`),'Audio formats must stay together in a loose mixed directory');
 await expect(playbackBox('01.flac')).toBeChecked();
 await expect(playbackBox('02.FLAC')).toBeChecked();
 await expect(playbackBox('01.mp3')).not.toBeChecked();
 await imports.getByRole('button',{name:'Select MP3',exact:true}).click();
 await expect(playbackBox('01.flac')).not.toBeChecked();
 await expect(playbackBox('02.mp3')).toBeChecked();
 await expect(imports.getByLabel('Track number for 01.mp3',{exact:true})).toHaveAttribute('placeholder','1');
 await imports.getByRole('button',{name:'Select FLAC',exact:true}).click();
 const discFor=name=>imports.getByLabel(`Disc for ${name}`,{exact:true});
 const discCount=imports.getByLabel('Number of discs',{exact:true});
 const applyDiscs=imports.getByRole('button',{name:'Apply disc count'});
 await discCount.fill('2');await applyDiscs.click();
 const trackNumber=name=>imports.getByRole('spinbutton',{name:`Track number for ${name}`,exact:true});
 await trackNumber('02.FLAC').fill('7');
 await trackNumber('02.FLAC').press('Tab');
 await trackNumber('01.flac').fill('7');
 await trackNumber('01.flac').press('Enter');
 await expect(playbackRows.first()).toHaveAccessibleName('Use for album playback: 01.flac');
 await trackNumber('01.flac').fill('9');
 await trackNumber('01.flac').press('Enter');
 await expect(playbackRows.first()).toHaveAccessibleName('Use for album playback: 02.FLAC');
 await expect(trackNumber('01.flac')).toBeFocused();
 await trackNumber('01.flac').fill('1');
 await trackNumber('01.flac').press('Tab');
 await expect(playbackRows.first()).toHaveAccessibleName('Use for album playback: 01.flac');
 await discFor('01.flac').selectOption('2');
 await imports.getByRole('button',{name:'Apply to below: Disc 2 from 01.flac',exact:true}).click();
 await expect(discFor('02.FLAC')).toHaveValue('2');
 await expect(playbackBox('02.mp3')).not.toBeChecked();
 await discCount.fill('1');await applyDiscs.click();
 await expect(discFor('02.FLAC')).toHaveValue('1');
 await discFor('01.flac').selectOption({label:'New disc (2)'});
 await imports.getByRole('button',{name:'Apply to below: Disc 2 from 01.flac',exact:true}).click();
 await discFor('01.flac').selectOption('1');
 const moveUp=imports.getByRole('button',{name:'Move 02.FLAC up',exact:true});
 const moveDown=imports.getByRole('button',{name:'Move 02.FLAC down',exact:true});
 await expect(moveDown).toBeDisabled();
 await moveUp.click();
 await expect(playbackRows.first()).toHaveAccessibleName('Use for album playback: 02.FLAC');
 await expect(moveDown).toBeFocused();
 await moveDown.press('Space');
 await expect(playbackRows.first()).toHaveAccessibleName('Use for album playback: 01.flac');
 await moveUp.click();
 await expect.poll(async()=>(await intakeDrafts()).some(draft=>{const pkg=draft.packages?.[0]?.package;return pkg?.files.filter(file=>/\.flac$/i.test(file.name)).map(file=>file.name).join()==='02.FLAC,01.flac'&&pkg.album_tracks?.some(track=>track.file_name==='02.FLAC'&&track.disc_position===2&&track.display_position===7);})).toBe(true);
 await page.reload();
 await openFileTree(imports);
 await expect(playbackRows.first()).toHaveAccessibleName('Use for album playback: 02.FLAC');
 await expect(discFor('01.mp3')).toHaveCount(0);
 await expect(trackNumber('01.mp3')).toHaveCount(0);
 await expect(trackNumber('02.FLAC')).toHaveValue('7');
 await expect(discFor('02.FLAC')).toHaveValue('2');
 for(const width of [390,1440]){
  await page.setViewportSize({width,height:1000});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Mixed import review overflows');
 }
 await imports.getByLabel('Store',{exact:true}).fill('Separate inbox purchase');
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const inboxId=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,inboxId),{timeout:60000}).toBe('completed');
 const inboxItem=await page.evaluate(async id=>{const data=await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json();return data.items[0].import.target_item_id;},inboxId);
 assert.notEqual(inboxItem,item);
 const inboxCapture=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),inboxItem);
 assert.equal(inboxCapture.specimens.length,0);assert.equal(inboxCapture.captures[0].association.specimen_id,null);assert.equal(inboxCapture.captures[0].purchase.store,'Separate inbox purchase');
 assert.deepEqual(inboxCapture.captures[0].representations[0].files.map(file=>file.name).sort(),
  ['01.flac','01.mp3','02.FLAC','02.mp3','booklet.txt','cover.png','receipt.txt']);
 assert.deepEqual(inboxCapture.captures[0].representations[0].files.filter(file=>/\.flac$/i.test(file.name)).map(file=>file.name),
  ['02.FLAC','01.flac'],'Preservation must use the reviewed file order');
 const inboxTracks=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks,inboxItem);
 assert.deepEqual(inboxTracks.map(track=>[track.disc_position,track.display_position,track.audio_format.codec]),[[1,1,'flac'],[2,7,'flac']]);
 await writeFile(path.join(fixture,'broken.mp3'),Buffer.from('not a valid audio stream'));
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(path.join(fixture,'broken.mp3'));
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const brokenId=new URL(page.url()).searchParams.get('item');let broken;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,brokenId);broken=members[0]?.import?.target_item_id;return members[0]?.import?.phase==='published'?members[0].import.outcome:'waiting_for_preservation';},{timeout:60000}).toBe('failed');
 await page.goto(`${origin}/?collection=${collection}&item=${broken}`);
 // The track panel finishes loading with no playable tracks.
 const trackPanel=page.getByRole('region',{name:/^(Prepare audio|Album tracks)$/});
 await trackPanel.locator('p.notice').waitFor();
 await expect(trackPanel.getByRole('status')).toHaveCount(0);
 assert.equal(await page.getByRole('button',{name:'Inspect CD structure'}).count(),0);
 // Same-format copies still require a choice after tag analysis.
 await copyFile(path.join(fixture,'01.flac'),path.join(fixture,'alternate.flac'));
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(['01.flac','alternate.flac'].map(name=>path.join(fixture,name)));
 await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const alternateIntake=new URL(page.url()).searchParams.get('item');let alternate;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,alternateIntake);alternate=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:60000}).toBe('succeeded');
 await page.goto(`${origin}/?collection=${collection}&item=${alternate}`);
 await expect(page.locator('.album-playback-notice')).toBeVisible();
 await expect(tracks.locator('.music-track')).not.toHaveCount(0);
 const attentionBefore=await page.evaluate(async collection=>(await(await fetch(`/api/v1/items?collection_id=${collection}&filter=album_playback_needs_attention`)).json()).items.map(item=>item.id),collection);
 assert.ok(attentionBefore.includes(alternate),'ambiguous purchase missing from playback attention results');

 const beforeAmbiguous=await page.evaluate(()=>Object.entries(localStorage).filter(([key])=>key.startsWith('collector.music-queue.v1.')));
 // The server refuses album playback until alternate encodings are chosen.
 const refusedAlbum=page.waitForResponse(r=>new URL(r.url()).pathname===`/api/v1/items/${alternate}/client-playback`);
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 assert.equal((await refusedAlbum).ok(),false,'ambiguous album playback was accepted');
 await expect(tracks.getByRole('alert')).toBeVisible();
 assert.deepEqual(await page.evaluate(()=>Object.entries(localStorage).filter(([key])=>key.startsWith('collector.music-queue.v1.'))),beforeAmbiguous);
 await tracks.locator('.music-track').last().getByRole('button',{name:/^Play track /}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.02&&audio.currentSrc.endsWith('/1');}return false;});
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await page.getByRole('button',{name:'Album management',exact:true}).click();await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();
 await editor.getByRole('button',{name:'Music',exact:true}).click();
 for(const [index,included] of [[0,true],[1,false]]){
  await clickAction(editor,'Edit track details',index);
  await editor.getByLabel('Album playback and lists',{exact:true}).selectOption(included?'include':'exclude');
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
 await expectQueue(player,1);
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 // WAV is a digital original; the normal intake option prepares native-format FLAC.
 const wavFile=path.join(scratch,'03.wav');
 const cleanWav=await readFile(path.join(root,'crates/collector-store/tests/fixtures/digital-audio/03.wav'));
 const suffixedWav=Buffer.concat([cleanWav,Buffer.from('ID3\xffuninterpreted trailer\xff','latin1')]);
 await writeFile(wavFile,suffixedWav);
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(wavFile);
 await expect(imports.getByRole('button',{name:'Start import',exact:true})).toBeVisible();
 await imports.getByLabel('Store',{exact:true}).fill('WAV store');
 await imports.getByLabel('Audio preparation after import',{exact:true}).selectOption('flac');
 await expect(imports.getByLabel('Physical copy label')).toHaveCount(0);
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));
 const wavIntake=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,wavIntake),{timeout:60000}).toBe('completed');
 await imports.getByRole('button',{name:'Review import',exact:true}).click();
 const wavReview=page.getByRole('dialog',{name:'Import review',exact:true});
 // Trailing bytes outside the WAV RIFF segment are reported as an import limitation.
 const wavImport=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items[0].import.import_id,wavIntake);
 const wavLimitations=(await page.evaluate(async id=>(await(await fetch(`/api/v1/ingests/${id}`)).json()),wavImport)).import.limitations;
 assert.equal(wavLimitations.length,1);
 const wavSteps=wavReview.locator('.ingest-steps p');
 await expect(wavSteps).toHaveCount(1);
 await page.reload();
 await imports.getByRole('button',{name:'Review import',exact:true}).click();
 await expect(wavSteps).toHaveCount(1);
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 
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
 await page.reload();await expect(tracks.locator('.music-track[data-track-position="1"] .track-title strong')).toHaveAttribute('title','03.wav');
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 
 // Preparation remains available later from the real Music editor.
 await page.setViewportSize({width:1440,height:1000});
 await page.goto(`${origin}/imports?collection=${collection}`);
 await imports.getByLabel("Choose one package's files").setInputFiles(wavFile);
 await expect(imports.getByLabel('Audio preparation after import',{exact:true})).toHaveValue('default');
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));
 const laterIntake=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,laterIntake),{timeout:60000}).toBe('completed');
 const laterItem=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items[0].import.target_item_id,laterIntake);
 await page.goto(`${origin}/?collection=${collection}&item=${laterItem}`);
 // The strict WAV reader plays the declared 24-bit segment before any FLAC is prepared.
 // Nothing is playing after navigation, so the format is chosen in Account.
 await page.getByRole('button',{name:'Account & security',exact:true}).click();
 const accountDialog=page.getByRole('dialog',{name:'Account & security'});
 await accountDialog.getByRole('button',{name:'Playback',exact:true}).click();
 await accountDialog.getByLabel('Playback format').selectOption('high_bandwidth');
 await accountDialog.getByRole('button',{name:'Close account and security',exact:true}).click();
 const liveWav=page.waitForResponse(r=>r.request().method()==='GET'&&r.url().includes('/music-stream/')&&r.url().includes('mode=high_bandwidth'));
 await tracks.getByRole('button',{name:'Play track 1: 03.wav',exact:true}).click();
 const converted=await liveWav;
 assert.equal(converted.status(),200);
 assert.equal(converted.headers()['accept-ranges'],'none');
 await expect.poll(()=>player.locator('audio').evaluate(audio=>!audio.paused&&audio.currentTime>.02)).toBe(true);
 await player.getByRole('button',{name:'Pause',exact:true}).click();
 await player.getByLabel('Playback format').selectOption('high_bandwidth');
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
 await expect(tracks.locator('.music-track .track-title strong[title="WAV title"]')).toBeVisible();
 assert.deepEqual((await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks,taggedItem)).map(track=>[track.display_position,track.disc_position]),[[7,2]]);
 await expect(page.getByRole('region',{name:'Selected entry'}).getByRole('img').first()).toBeVisible();
 const downloadEvent=page.waitForEvent('download');await tracks.locator('a.track-format.ready[href*="/music-exports/"]').first().click();
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
 const run=analysis.getByRole('button',{name:'Analyze audio',exact:true});
 await expect(run).toBeEnabled();
 const analysisPost=page.waitForResponse(response=>response.url().endsWith('/operations')&&response.request().method()==='POST').then(async response=>({ok:response.ok(),body:await response.json()})).catch(error=>({ok:false,error:String(error)}));
 await run.click();
 const analysisResponse=await analysisPost;assert.ok(analysisResponse.ok,JSON.stringify(analysisResponse));const analysisReceipt=analysisResponse.body;assert.ok(analysisReceipt.job_id);
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/jobs/${id}`)).json()).state,analysisReceipt.job_id),{timeout:45000}).toBe('completed');
 const analyzedSource=beforeAnalysis.captures[0].representations.find(rep=>rep.role==='preservation_master');
 let analysisObservation;
 await expect.poll(async()=>{const history=await page.evaluate(async({capture,representation})=>(await(await fetch(`/api/v1/captures/${capture}/operations?representation_id=${representation}&limit=100`)).json()).items,{capture:beforeAnalysis.captures[0].id,representation:analyzedSource.id});analysisObservation=history.find(summary=>Array.isArray(summary.result?.file_tracks?.tracks));return analysisObservation?.result.file_tracks.tracks.length;},{timeout:45000}).toBe(1);
 await expect(analysis.locator(`[id="processing-result-${analysisObservation.observation_id}"]`)).toBeVisible({timeout:45000});
 await editor.getByRole('button',{name:'Music',exact:true}).click();await expect(editor.locator('.music-track .track-title strong[title="WAV title"]')).toBeVisible();
 await editor.getByRole('button',{name:'Close curator'}).click();
 const afterAnalysis=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),taggedItem);
 assert.deepEqual(afterAnalysis.captures,beforeAnalysis.captures);assert.equal(afterAnalysis.primary_artwork,beforeAnalysis.primary_artwork);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 
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
 await steamImports.getByRole('button',{name:'▸ Large review',exact:true}).click();
 const fileLabels=()=>steamImports.locator('.import-tree-file input[aria-label^="Use for album playback:"]').evaluateAll(inputs=>inputs.map(input=>input.getAttribute('aria-label')));
 const firstPage=await fileLabels();assert.equal(firstPage.length,50);
 const largeDisc=name=>steamImports.getByLabel(`Disc for Large review/${name}.flac`,{exact:true});
 await largeDisc('0048').selectOption('1');
 await largeDisc('0049').selectOption({label:'New disc (2)'});
 await expect(steamImports.getByLabel('Track number for Large review/0049.flac',{exact:true})).toHaveAttribute('placeholder','1');
 await steamImports.getByRole('button',{name:'Apply to below: Disc 2 from Large review/0049.flac',exact:true}).click();
 await expect(largeDisc('0048')).toHaveValue('1');
 await steamImports.getByRole('button',{name:'Next entries',exact:true}).click();
 const secondPage=await fileLabels();assert.equal(secondPage.length,50);
 assert.equal(new Set([...firstPage,...secondPage]).size,100);
 await expect(largeDisc('0050')).toHaveValue('2');
 await largeDisc('0051').selectOption({label:'New disc (3)'});
 await steamImports.getByRole('button',{name:'Apply to below: Disc 3 from Large review/0051.flac',exact:true}).click();
 await expect(largeDisc('0050')).toHaveValue('2');
 await expect(largeDisc('0099')).toHaveValue('3');
 // Steam-style format wrappers: real folder discovery, review and publication.
 const steam=path.join(scratch,'Steam soundtracks');
 for(const folder of ['Game Name OST/FLAC/Disc 1','Game Name OST/MP3/Disc 1','Game Name OST/WAV/Disc 1','Game Name OST/Booklet','Other Album/FLAC/Disc 2','Other Album/Bonus/Disc 2','Wild Album/All FLAC','Wild Album/DLC MP3'])await mkdir(path.join(steam,folder),{recursive:true});
 await copyFile(path.join(fixture,'01.flac'),path.join(steam,'Game Name OST/FLAC/Disc 1/01.flac'));
 await copyFile(path.join(fixture,'02.mp3'),path.join(steam,'Game Name OST/MP3/Disc 1/01.mp3'));
 const untaggedFlac=path.join(steam,'Other Album/FLAC/01.flac');
 const encoded=spawnSync(flac,['--silent','--output-name',untaggedFlac,path.join(root,'crates/collector-store/tests/fixtures/digital-audio/03.wav')],{encoding:'utf8'});
 assert.equal(encoded.status,0,encoded.stderr);
 for(let track=2;track<=10;track++){
  await copyFile(untaggedFlac,path.join(steam,'Other Album/FLAC',`${String(track).padStart(2,'0')}.flac`));
 }
 // This file has an embedded track number of 1. The folder move puts it last.
 await copyFile(path.join(fixture,'01.flac'),path.join(steam,'Other Album/Bonus/11.flac'));
 await copyFile(path.join(fixture,'01.flac'),path.join(steam,'Other Album/Bonus/Disc 2/02.flac'));
 await copyFile(untaggedFlac,path.join(steam,'Other Album/FLAC/Disc 2/01.flac'));
 await copyFile(wavFile,path.join(steam,'Game Name OST/WAV/Disc 1/01.wav'));
 for(const name of ['01.flac','02.flac'])await copyFile(untaggedFlac,path.join(steam,'Wild Album/All FLAC',name));
 await copyFile(path.join(fixture,'02.mp3'),path.join(steam,'Wild Album/DLC MP3/01.mp3'));
 await copyFile(path.join(fixture,'cover.png'),path.join(steam,'Game Name OST/cover.png'));
 await writeFile(path.join(steam,'Game Name OST/Booklet/booklet.txt'),'PRIVATE STEAM BOOKLET');
 await page.goto(`${origin}/imports?collection=${collection}`);
 await chooseFolder(page,steamImports.getByLabel('Choose folders',{exact:true}),steam);
 await expect(steamImports.getByRole('button',{name:'Start import',exact:true})).toBeVisible();
 const packageRows=steamImports.locator('.intake-review-row');await expect(packageRows).toHaveCount(3);
 const flacRow=packageRows.filter({has:page.getByText('Game Name OST',{exact:true})});
 const wildRow=packageRows.filter({has:page.getByText('Wild Album',{exact:true})});
 const otherRow=packageRows.filter({has:page.getByText('Other Album',{exact:true})});
 await expect(flacRow).toHaveCount(1);await expect(wildRow).toHaveCount(1);await expect(otherRow).toHaveCount(1);
 await expect(steamImports.getByRole('button',{name:'Start import',exact:true})).toBeDisabled();
 await expect(wildRow.locator('p.notice')).toBeVisible();
 await openFileTree(wildRow);
 await wildRow.getByRole('button',{name:'Select FLAC',exact:true}).click();
 await wildRow.getByLabel(/Use for album playback: .*All FLAC\/01.flac$/).uncheck();
 await wildRow.getByLabel('Number of discs',{exact:true}).fill('2');
 await wildRow.getByRole('button',{name:'Apply disc count'}).click();
 await wildRow.getByLabel(/Disc for .*All FLAC\/02.flac$/).selectOption('2');
 await expect(wildRow.getByLabel(/Track number for .*All FLAC\/02.flac$/)).toHaveAttribute('placeholder','1');
 await openFileTree(otherRow);
 const otherTrack=name=>otherRow.getByRole('spinbutton',{name:new RegExp(`^Track number for .*/Other Album/${name}\\.flac$`)});
 const moveBonusDown=otherRow.getByRole('button',{name:/^Move folder .*Other Album\/Bonus down$/});
 const moveBonusUp=otherRow.getByRole('button',{name:/^Move folder .*Other Album\/Bonus up$/});
 await moveBonusDown.click();
 await expect(moveBonusUp).toBeFocused();
 await expect(otherTrack('FLAC/01')).toHaveAttribute('placeholder','1');
 await expect(otherTrack('Bonus/11')).toHaveAttribute('placeholder','11');
 await otherTrack('FLAC/01').fill('10');
 await otherTrack('FLAC/01').press('Enter');
 await expect(otherTrack('FLAC/01')).toBeFocused();
 await expect(otherTrack('FLAC/02')).toHaveAttribute('placeholder','1');
 await expect(otherTrack('FLAC/10')).toHaveAttribute('placeholder','9');
 await expect(otherTrack('Bonus/11')).toHaveAttribute('placeholder','11');
 // A saved draft restores choices independently of the browser's file handles.
 await expect.poll(async()=>(await intakeDrafts()).some(draft=>{const tracks=(draft.packages||[]).flatMap(entry=>entry.package.album_tracks||[]);return tracks.some(track=>track.file_name.endsWith('All FLAC/02.flac')&&track.disc_position===2)&&tracks.some(track=>track.file_name.endsWith('Other Album/FLAC/01.flac')&&track.display_position===10);})).toBe(true);
 await page.reload();await expect(packageRows).toHaveCount(3);
 await openFileTree(wildRow);
 await expect(wildRow.getByLabel(/Use for album playback: .*All FLAC\/01.flac$/)).not.toBeChecked();
 await expect(wildRow.getByLabel(/Disc for .*All FLAC\/02.flac$/)).toHaveValue('2');
 await openFileTree(otherRow);
 await expect(moveBonusDown).toBeDisabled();
 await expect(otherTrack('FLAC/01')).toHaveValue('10');
 await expect(otherTrack('FLAC/02')).toHaveAttribute('placeholder','1');
 await chooseFolder(page,steamImports.getByLabel('Reselect folder for reviewed import',{exact:true}),steam);
 await openFileTree(flacRow);
 await expect(flacRow.getByLabel(/Curators only: .*Booklet\/booklet.txt/)).toBeChecked();
 await expect(flacRow.getByLabel(/Curators only: .*cover.png/)).not.toBeChecked();
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.setViewportSize({width:1440,height:1000});
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
 // The album lists only the copy it plays; the MP3 and WAV copies of the same
 // track are kept, collapsed, rather than shown as extra album tracks.
 const importsUrl=page.url();
 await page.goto(`${origin}/?collection=${collection}&item=${flacMember.import.target_item_id}`);
 const albumTracks=page.getByRole('region',{name:'Album tracks'});
 const otherCopies=albumTracks.locator('details.other-tracks');
 await expect(albumTracks.locator('.music-track')).toHaveCount(3);
 await expect(otherCopies.locator('.music-track')).toHaveCount(2);
 await expect(otherCopies.locator('.music-track').first()).toBeHidden();
 await otherCopies.locator(':scope > summary').click();
 await expect(otherCopies.locator('.music-track')).toHaveCount(2);
 await expect(otherCopies.getByRole('button',{name:/^Play track /}).first()).toBeEnabled();
 await expect(otherCopies.getByRole('button',{name:'Play original',exact:true})).toHaveCount(0);
 // One choice switches every track to another copy, and Automatic restores the
 // best-quality pick. Playback follows the saved choice.
 const playedCodecs=()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks.map(track=>track.audio_format.codec).join(),flacMember.import.target_item_id);
 const albumUses=albumTracks.getByRole('combobox',{name:'Album uses'});
 await expect(albumUses).toHaveValue('flac');
 await albumUses.selectOption('mp3');
 await expect.poll(playedCodecs).toBe('mp3');
 await expect(albumUses).toHaveValue('mp3');
 await expect(albumTracks.locator('.music-track')).toHaveCount(3);
 await expect(otherCopies.locator('.music-track')).toHaveCount(2);
 await albumTracks.getByRole('button',{name:'Use best quality automatically'}).click();
 await expect.poll(playedCodecs).toBe('flac');
 await expect(albumUses).toHaveValue('flac');
 await page.goto(importsUrl);
 const wildMember=steamMembers.find(m=>m.plan.default_title==='Wild Album');
 const selectedWild=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks,wildMember.import.target_item_id);
 assert.equal(selectedWild.length,1);assert.equal(selectedWild[0].disc_position,2);
 assert.equal(selectedWild[0].display_position,1);
 const otherMember=steamMembers.find(m=>m.plan.default_title==='Other Album');
 const selectedOther=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-playback`)).json()).tracks,otherMember.import.target_item_id);
 const otherAlbum=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),otherMember.import.target_item_id);
 const otherFiles=otherAlbum.captures[0].representations[0].files;
 const otherDiscOne=selectedOther.filter(track=>(track.disc_position??1)===1);
 const otherDiscTwo=selectedOther.filter(track=>track.disc_position===2);
 assert.deepEqual(selectedOther.map(track=>track.disc_position),[...Array(11).fill(1),2,2]);
 assert.deepEqual(otherDiscOne.map(track=>track.display_position),[1,2,3,4,5,6,7,8,9,10,11]);
 assert.deepEqual(otherDiscOne.map(track=>otherFiles[track.state.file_index].name.split('/').slice(-2).join('/')),
  ['FLAC/02.flac','FLAC/03.flac','FLAC/04.flac','FLAC/05.flac','FLAC/06.flac','FLAC/07.flac',
   'FLAC/08.flac','FLAC/09.flac','FLAC/10.flac','FLAC/01.flac','Bonus/11.flac']);
 assert.deepEqual(otherDiscTwo.map(track=>[track.display_position,otherFiles[track.state.file_index].name.split('/').slice(-3).join('/')]),
  [[1,'FLAC/Disc 2/01.flac'],[2,'Bonus/Disc 2/02.flac']]);
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
  await expect.poll(()=>continuationRequests).toBe(1);
 }finally{releasePage();}
 await expect(published).toHaveCount(2);

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
 const metadataWrites=[];page.on('request',request=>{if(request.method()==='PUT'&&/\/api\/v1\/items\/[^/]+\/metadata$/.test(new URL(request.url()).pathname))metadataWrites.push(new URL(request.url()).pathname.split('/')[4]);});
 await expect(saveAlbums).toBeDisabled();
 await albumEditor.getByRole('checkbox',{name:'Overwrite digital source'}).check();
 await albumEditor.getByRole('textbox',{name:'Digital source'}).fill('Steam');
 await albumEditor.getByRole('checkbox',{name:'Overwrite album artist'}).check();
 await albumEditor.getByRole('textbox',{name:'Album artist'}).fill('Steam soundtrack artists');
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
 
 await saveAlbums.click();
 await expect(albumEditor.getByRole('alert')).toContainText('Album changed while saving');
 assert.deepEqual([...metadataWrites].sort(),[steamMembers[1].import.target_item_id,remaining].sort(),'save must write exactly the two selected albums');
 await expect(albumEditor.getByRole('button',{name:'Retry remaining albums'})).toBeVisible();
 const updated=await readMetadata(steamMembers[1].import.target_item_id);
 assert.equal(updated.display.domain_fields.digital_source.value,'Steam');
 assert.equal(updated.display.subtitle.value,'Steam soundtrack artists');
 assert.equal((await readMetadata(remaining)).revision,initial[2].revision);
 const retryWrite=page.waitForResponse(response=>response.request().method()==='PUT'&&new URL(response.url()).pathname===`/api/v1/items/${remaining}/metadata`);
 await albumEditor.getByRole('button',{name:'Retry remaining albums'}).click();
 assert.ok((await retryWrite).ok(),'retry write failed');
 await expect(albumEditor.getByRole('button',{name:'Retry remaining albums'})).toHaveCount(0);
 await expect(albumEditor.getByRole('button',{name:'Done',exact:true})).toBeEnabled();
 await expect(albumEditor.getByRole('alert')).toHaveCount(0);
 assert.deepEqual(metadataWrites.slice(2),[remaining],'retry must write only the failed album');
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
 const savedList=page.waitForResponse(response=>{const url=new URL(response.url());return url.pathname==='/api/v1/intakes'&&!url.searchParams.has('cursor');});
 await steamImports.getByRole('button',{name:'Back to imports',exact:true}).click();
 const savedImports=steamImports.getByRole('region',{name:'Saved imports'});
 const newestSaved=(await (await savedList).json()).items[0];assert.equal(newestSaved.id,steamIntake);assert.equal(newestSaved.package_count,3);
 const savedRows=savedImports.locator('button.collection-row');
 await expect(savedRows).toHaveCount(2);
 await savedImports.getByRole('button',{name:'More imports',exact:true}).click();
 try{
  const loading=savedImports.getByRole('button',{name:'Loading imports…',exact:true});
  await expect(loading).toBeDisabled();
 }finally{releaseSaved();}
 await expect(savedRows).toHaveCount(4);
 await page.unroute('**/api/v1/intakes?*',shortSaved);
 // Immediate Library refresh exposes the newly imported version and its track.
 await page.goto(`${origin}/?collection=${collection}&item=${flacMember.import.target_item_id}`);
 await expect(page.getByRole('region',{name:'Album tracks'}).locator('.music-track .track-title strong[title="FLAC original"]')).toBeVisible();
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
