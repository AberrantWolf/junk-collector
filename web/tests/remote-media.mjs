import {stopProcess} from './support/runtime.mjs';
// Real core, separate media executable/Unix sockets, HTTPS frontend and HTML audio.
// HTTPS trust exceptions are scoped to this disposable self-signed test context;
// this is not evidence of public-CA deployment or actual iPhone behavior.
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {spawn,spawnSync} from './support/runtime.mjs';
import {createServer as netServer} from 'node:net';
import {createServer as tlsServer} from 'node:https';
import {request as httpRequest} from 'node:http';
import path from 'node:path';
import {chromium,expect} from './support/browser.mjs';
async function port(){const server=netServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const value=server.address().port;await new Promise(resolve=>server.close(resolve));return value;}
const root=path.resolve(import.meta.dirname,'../..');const scratch=await mkdtemp('/tmp/jc-media-');
const ports=await Promise.all([port(),port(),port(),port()]);const single=process.env.COLLECTOR_TEST_SINGLE_ORIGIN==='1';const upstream=`http://localhost:${ports[0]}`;const origin=single?`https://localhost:${ports[3]}`:upstream;const remote=`https://localhost:${ports[3]}/${single?'media/':''}`;
const config=path.join(scratch,'collector.toml');const authority=path.join(scratch,'auth/socket');const mediaSocket=path.join(scratch,'http/socket');await mkdir(path.dirname(mediaSocket));
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:${ports[0]}"\nprivate_bind = "127.0.0.1:${ports[1]}"\nlan_bind = "127.0.0.1:${ports[2]}"\nlocal_playback_enabled = false\nremote_media_allowed = false\nallowed_origins = ["${origin}"]\n[remote_media]\nbase_url = "${remote}"\nsocket = "${authority}"\n[exposure]\npublic_curation = true\npublic_administration = true\n`);
const binary=(process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server'));const password='Disposable remote-media fixture password';
const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
const children=[];let logs='';const start=(binary,args)=>{const child=spawn(binary,args,{stdio:['ignore','pipe','pipe']});children.push(child);child.stdout.on('data',b=>logs+=b);child.stderr.on('data',b=>logs+=b);return child;};
let browser,front;const observed=[];let rejectedGrant;const rejectFirst=process.env.COLLECTOR_TEST_MEDIA_REJECT_FIRST_READ==='1';
try{
 start(binary,['--config',config,'serve']);
 for(let n=0;;n++){try{if((await fetch(upstream+'/healthz')).ok)break;}catch{}if(n>150)throw new Error(logs);await new Promise(r=>setTimeout(r,100));}
 start((process.env.COLLECTOR_MEDIA_BINARY||path.join(root,'target/debug/collector-media')),['--http-socket',mediaSocket,'--authority-socket',authority,'--archive',path.join(scratch,'archive'),'--allowed-origin',origin]);
 front=tlsServer({key:await readFile(path.join(scratch,'state/tls/key.pem')),cert:await readFile(path.join(scratch,'state/tls/cert.pem'))},(request,response)=>{
  const url=new URL(request.url,remote);const media=single?url.pathname.startsWith('/media/'):true;const pathname=single&&media?url.pathname.slice(6):url.pathname;
  if(media&&(!['/read','/api/v1/identity'].includes(pathname)||!['GET','HEAD','OPTIONS'].includes(request.method))){response.writeHead(404).end();return;}
  if(media&&pathname==='/read'&&request.method==='GET'&&rejectFirst){const grant=url.searchParams.get('grant');if(!rejectedGrant)rejectedGrant=grant;if(grant===rejectedGrant){response.writeHead(401,{'access-control-allow-origin':origin,'cache-control':'no-store'}).end();return;}}
  const headers={...request.headers};if(media){delete headers.cookie;observed.push({path:pathname,cookie:headers.cookie,incomingCookie:request.headers.cookie,authorization:headers.authorization});}
  const forwarded=httpRequest({...media?{socketPath:mediaSocket}:{hostname:'127.0.0.1',port:ports[0]},path:pathname+url.search,method:request.method,headers},result=>{if(media)delete result.headers['set-cookie'];response.writeHead(result.statusCode,result.headers);result.pipe(response);});
  forwarded.on('error',()=>{if(!response.headersSent)response.writeHead(503).end();else response.destroy();});request.pipe(forwarded);response.on('close',()=>forwarded.destroy());
 });await new Promise(resolve=>front.listen(ports[3],'127.0.0.1',resolve));
 browser=await chromium.launch({mode:process.env.COLLECTOR_TEST_BROWSER||'chrome',headless:true});
 const context=await browser.newContext({ignoreHTTPSErrors:true});await context.addInitScript(()=>{window.mediaActions={};const session=navigator.mediaSession;const original=session.setActionHandler.bind(session);session.setActionHandler=(action,handler)=>{window.mediaActions[action]=handler;original(action,handler);};});const page=await context.newPage();page.setDefaultTimeout(20000);const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});await admin.getByLabel('Collection name').fill('Remote audio');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>{const data=await(await fetch('/api/v1/collection-search?limit=50')).json();return data.items.find(c=>c.name==='Remote audio').id;});
 await page.goto(`${origin}/imports?collection=${collection}`);const imports=page.getByRole('region',{name:'Imports workspace'});
 await imports.getByLabel("Choose one package's files").setInputFiles(['01.flac','02.mp3','cover.png'].map(name=>path.join(root,'crates/collector-store/tests/fixtures/digital-audio',name)));await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await page.waitForURL(url=>url.searchParams.has('item'));const intake=new URL(page.url()).searchParams.get('item');let item;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);item=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:45000}).toBe('succeeded');
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);await page.getByRole('region',{name:'Album tracks'}).waitFor();
 // Force the ordinary-HTTPS browser path; no WebTransport/worker is available.
 await page.evaluate(()=>{window.WebTransport=undefined;document.querySelector('audio').loop=true;});
 await page.getByRole('region',{name:'Album tracks'}).getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('audio');return audio&&!audio.paused&&audio.currentTime>.02&&audio.currentSrc.includes('/read?grant=');});
 const player=page.getByRole('region',{name:'Music player'});await expect(player).toContainText('Direct HTTPS playback');
 await page.waitForFunction(()=>navigator.mediaSession.metadata?.title==='FLAC original');
 assert.deepEqual(await page.evaluate(()=>{const m=navigator.mediaSession.metadata;return {title:m.title,artist:m.artist,album:m.album,state:navigator.mediaSession.playbackState};}),{title:'FLAC original',artist:'Digital artist',album:'Purchased album',state:'playing'});
 const art=await page.evaluate(()=>navigator.mediaSession.metadata.artwork[0]?.src);assert.ok(art?.startsWith(origin+'/api/v1/collections/'));assert.equal(await page.evaluate(async url=>(await fetch(url)).status,art),200);
 await page.evaluate(()=>window.mediaActions.pause({action:'pause'}));await page.waitForFunction(()=>document.querySelector('audio').paused&&navigator.mediaSession.playbackState==='paused');
 await page.evaluate(()=>window.mediaActions.play({action:'play'}));await page.waitForFunction(()=>!document.querySelector('audio').paused&&navigator.mediaSession.playbackState==='playing');
 const seek=await page.evaluate(()=>{const a=document.querySelector('audio'),target=a.duration*.4;window.mediaActions.seekto({action:'seekto',seekTime:target});return {position:a.currentTime,target};});assert.ok(Math.abs(seek.position-seek.target)<.02);
 await page.evaluate(()=>window.mediaActions.nexttrack({action:'nexttrack'}));await page.waitForFunction(()=>navigator.mediaSession.metadata?.title==='MP3 original'&&!document.querySelector('audio').paused);
 await page.evaluate(()=>window.mediaActions.previoustrack({action:'previoustrack'}));await page.waitForFunction(()=>navigator.mediaSession.metadata?.title==='FLAC original'&&!document.querySelector('audio').paused);
 // Exercise natural completion, then keep the following short fixture looping.
 await page.locator('audio').evaluate(a=>{a.addEventListener('ended',()=>{a.loop=true;},{once:true});a.loop=false;a.currentTime=Math.max(0,a.duration-.01);});
 await page.waitForFunction(()=>navigator.mediaSession.metadata?.title==='MP3 original'&&!document.querySelector('audio').paused);
 const stopped=await page.evaluate(()=>{window.mediaActions.stop({action:'stop'});const a=document.querySelector('audio');return {metadata:navigator.mediaSession.metadata,paused:a.paused,src:a.getAttribute('src')};});assert.deepEqual(stopped,{metadata:null,paused:true,src:null});
 await player.getByRole('button',{name:'Play',exact:true}).click();await page.waitForFunction(()=>navigator.mediaSession.metadata?.title==='MP3 original'&&!document.querySelector('audio').paused);
 const duration=await page.locator('audio').evaluate(a=>a.duration);assert.ok(duration>0);await page.locator('audio').evaluate((a,d)=>{a.currentTime=d*.7;},duration);await page.waitForFunction(d=>document.querySelector('audio').currentTime>=d*.6,duration);await page.locator('audio').evaluate(a=>a.currentTime=.01);
 const track=await page.evaluate(()=>JSON.parse(localStorage.getItem(Object.keys(localStorage).find(k=>k.startsWith('collector.music-queue.v1.')))).tracks[0]);
 const cookie=(await context.cookies()).map(c=>`${c.name}=${c.value}`).join('; ');assert.equal((await fetch(`${upstream}/api/v1/items/${track.itemId}/representations/${track.representationId}/${track.fileIndex}`,{headers:{cookie}})).status,503);
 await page.goto(origin+'/remote-media');await page.getByRole('button',{name:'Test remote access',exact:true}).click();await page.getByText('Authenticated early and late media reads confirmed',{exact:false}).waitFor();await expect(page.getByRole('list')).toContainText('external reachability unproven');
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);await page.getByRole('region',{name:'Music player'}).waitFor();await page.evaluate(()=>{window.WebTransport=undefined;document.querySelector('audio').loop=true;});await page.getByRole('region',{name:'Music player'}).getByRole('button',{name:'Play',exact:true}).click();await page.waitForFunction(()=>{const audio=document.querySelector('audio');return audio&&!audio.paused&&audio.currentTime>.02;});
 const detached=await page.locator('audio').elementHandle();assert.equal(await page.evaluate(async()=>(await fetch('/api/v1/auth/logout',{method:'POST'})).status),204);await page.getByText('Your session expired or was revoked.',{exact:false}).waitFor();assert.deepEqual(await detached.evaluate(a=>({paused:a.paused,src:a.getAttribute('src')})),{paused:true,src:null});await detached.dispose();assert.equal(await page.evaluate(()=>navigator.mediaSession.metadata),null);assert.equal(await page.evaluate(()=>Object.values(window.mediaActions).every(handler=>handler===null)),true);
 assert.ok(observed.some(r=>r.path==='/read'));assert.ok(observed.every(r=>!r.cookie&&!r.authorization),'browser sent account credentials to media');if(!single)assert.ok(observed.every(r=>!r.incomingCookie),'cross-origin request carried a login cookie');if(rejectFirst)assert.ok(rejectedGrant,'lease-loss fixture was not exercised');assert.deepEqual(errors,[]);
 console.log(`Remote HTTPS playback (${single?'one origin with a media path':'separate origins'}) passed in Chrome ${await browser.version()}: real separate media executable/Unix sockets, actual FLAC/MP3 playback/seeks, OS media metadata/artwork/actions and automatic track transition, control media refusal, live logout/source release, no browser credential forwarding, synthetic challenge and real early/late range test. Fixture TLS only; iPhone Safari/cellular remains unqualified.`);
}catch(error){console.error(logs);throw error;}finally{
 await browser?.close();if(front){front.closeAllConnections();await new Promise(resolve=>front.close(resolve));}
 for(const child of children.reverse())await stopProcess(child);
 await rm(scratch,{recursive:true,force:true});
}
