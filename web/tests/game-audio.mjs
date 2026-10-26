import {stopProcess} from './support/runtime.mjs';
// Actual browser setup, mixed-mode game intake and listening against an isolated server.
import {chromium,expect,fixtureCertificateArgs} from './support/browser.mjs';
import {spawn,spawnSync} from './support/runtime.mjs';
import {mkdtemp,mkdir,writeFile,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
import {clickAction} from './actions.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const flacPath=process.env.COLLECTOR_TEST_FLAC||spawnSync('which',['flac'],{encoding:'utf8'}).stdout?.trim();
assert.ok(flacPath,'Install flac or set COLLECTOR_TEST_FLAC before game-audio acceptance');
assert.equal(spawnSync(flacPath,['--version'],{timeout:10000}).status,0,'The test FLAC encoder must be executable');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-game-audio-'));
const config=path.join(scratch,'collector.toml');
const password=randomBytes(24).toString('hex');
const origin='https://localhost:19883';
const fixture=path.join(scratch,'Game soundtrack');await mkdir(fixture);
let files,trackCount;
if(process.env.COLLECTOR_GAME_CUE){
 const cue=path.resolve(process.env.COLLECTOR_GAME_CUE);const text=await readFile(cue,'utf8');assert.ok(text.length<=1024*1024);
 files=[cue,...[...text.matchAll(/^\s*FILE "([^"\n]+)" BINARY\s*$/gm)].map(match=>path.join(path.dirname(cue),match[1]))];trackCount=[...text.matchAll(/^\s*TRACK \d+ AUDIO\s*$/gm)].length;assert.ok(trackCount>0);
}else{
await writeFile(path.join(fixture,'game.cue'),'FILE "disc.bin" BINARY\n TRACK 01 MODE2/2352\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n INDEX 01 00:01:00\n TRACK 03 AUDIO\n INDEX 01 00:06:00\n');
const pcm=Buffer.alloc(11*75*2352);for(let frame=0;frame<pcm.length/4;frame++){pcm.writeInt16LE(Math.round(Math.sin(frame*2*Math.PI*440/44100)*4096),frame*4);pcm.writeInt16LE(Math.round(Math.sin(frame*2*Math.PI*660/44100)*4096),frame*4+2);}await writeFile(path.join(fixture,'disc.bin'),pcm);
files=['game.cue','disc.bin'].map(file=>path.join(fixture,file));trackCount=2;
}
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19881"\nprivate_bind = "127.0.0.1:19882"\nlan_bind = "127.0.0.1:19883"\nlocal_playback_bind = "127.0.0.1:0"\nauthentication_origin = "${origin}"\nallowed_origins = ["${origin}"]\ntls_names = ["localhost", "127.0.0.1"]\nendpoints = ["${origin}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n[encoders]\nflac = ${JSON.stringify(flacPath)}\n[inboxes]\npurchases = "${scratch}"\n`);
const binary=(process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server'));
const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let logs='';const server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
let browser,page;
try{
 for(let i=0;i<300;i++){try{if((await fetch('http://127.0.0.1:19881/healthz')).ok)break;}catch{}if(server.exitCode!==null)throw Error(logs);await new Promise(r=>setTimeout(r,100));}
 browser=await chromium.launch({mode:process.env.COLLECTOR_TEST_BROWSER||'chrome',args:fixtureCertificateArgs(await readFile(path.join(scratch,'state/tls/cert.pem'),'utf8')),headless:true});
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);
 page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Game shelf');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.games');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>{const data=await(await fetch('/api/v1/collection-search?limit=50')).json();return data.items.find(c=>c.name==='Game shelf').id;});
 await page.goto(`${origin}/imports?collection=${collection}`);const imports=page.getByRole('region',{name:'Imports workspace'});
 await imports.getByLabel("Choose one package's files").setInputFiles(files);
 await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
 const intake=new URL(page.url()).searchParams.get('item');let item;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);item=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:process.env.COLLECTOR_GAME_CUE?600000:60000}).toBe('succeeded');
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);const tracks=page.getByRole('region',{name:'Album tracks'});
 await expect(tracks.locator('.music-track')).toHaveCount(trackCount);await tracks.getByText('2. Track 02',{exact:true}).waitFor();
 await page.getByRole('button',{name:'Edit / attach files',exact:true}).click();const editor=page.getByRole('dialog',{name:'Edit entry'});await editor.getByRole('button',{name:'Music',exact:true}).click();
 await clickAction(editor,'Edit track details',0);await editor.getByLabel('Track title',{exact:true}).fill('Game theme');await editor.getByRole('button',{name:'Save track details',exact:true}).click();await editor.getByText('2. Game theme',{exact:true}).waitFor();
 await editor.getByRole('button',{name:'Prepare missing FLAC tracks',exact:true}).click();await expect.poll(async()=>{const rows=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/client-tracks`)).json()),item);return rows.filter(row=>row.profile?.codec==='flac'&&row.state.status==='available').length;},{timeout:process.env.COLLECTOR_GAME_CUE?600000:60000}).toBe(trackCount);
 await editor.getByRole('button',{name:'Close curator'}).click();await page.reload();await tracks.getByText('2. Game theme',{exact:true}).waitFor();
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();const player=page.getByRole('region',{name:'Music player'});
 await page.waitForFunction(()=>{const a=document.querySelector('.music-player audio');return a&&!a.paused&&a.currentTime>.05;});await player.locator('audio').evaluate(a=>a.currentTime=2);await page.waitForFunction(()=>document.querySelector('.music-player audio')?.currentTime>2.05);
 await expect(player.getByRole('button',{name:`Queue (${trackCount})`,exact:true})).toBeVisible();await player.getByRole('button',{name:'Pause',exact:true}).click();
 await clickAction(tracks,'Add album to list');const chooser=page.getByRole('dialog',{name:'Add to list',exact:true});await chooser.getByLabel('New list name').fill('Game playlist');await chooser.getByRole('button',{name:'Append to list',exact:true}).click();await chooser.getByRole('button',{name:'Open list',exact:true}).click();
 const lists=page.getByRole('region',{name:'Collection lists'});await expect(lists.locator('.saved-list-entries li')).toHaveCount(trackCount);await lists.getByRole('button',{name:'Play playlist',exact:true}).click();await page.waitForFunction(()=>{const a=document.querySelector('.music-player audio');return a&&!a.paused&&a.currentTime>.05;});
 const songs=await page.evaluate(async id=>await(await fetch(`/api/v1/collections/${id}/music-search?search=Game%20theme`)).json(),collection);assert.equal(songs.items.length,1);assert.equal(songs.items[0].sources[0].track_position,2);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.screenshot({path:path.join(scratch,'game-audio-mobile.png'),fullPage:true});
 assert.deepEqual(errors,[]);console.log(`Game CD audio browser acceptance passed (Chrome ${await browser.version()}): setup, mixed-mode import, title edit/reload, real FLAC preparation, playback/seek, audio-only playlist, source-bound search and mobile layout. Artifacts: ${scratch}`);
}catch(error){console.error(logs);console.error('Browser state:',page?.url(),await page?.locator('body').innerText({timeout:2000}).catch(()=>''));await page?.screenshot({path:path.join(scratch,'failure.png'),fullPage:true}).catch(()=>{});throw error;}finally{await browser?.close();await stopProcess(server);}
