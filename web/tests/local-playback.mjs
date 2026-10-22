// Actual production app, server, worker, QUIC listener and HTML media element.
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createServer} from 'node:net';
import {spawn,spawnSync} from 'node:child_process';
import {chromium} from '@playwright/test';
async function port(){const server=createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const port=server.address().port;await new Promise(r=>server.close(r));return port;}
const ports=await Promise.all([port(),port(),port()]);const origin=`http://localhost:${ports[0]}`;
const scratch=await mkdtemp(path.join(tmpdir(),'collector-local-playback-'));const config=path.join(scratch,'collector.toml');
const binary=path.resolve(import.meta.dirname,'../../target/debug/collector-server');const password='Local-player-test-only-password';
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:${ports[0]}"\nprivate_bind = "127.0.0.1:${ports[1]}"\nlan_bind = "127.0.0.1:${ports[2]}"\nlocal_playback_bind = "127.0.0.1:0"\nremote_media_allowed = false\nallowed_origins = ["${origin}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n`);
const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let logs='';const server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stdout.on('data',d=>logs+=d);server.stderr.on('data',d=>logs+=d);
let browser;
try{
 for(let n=0;;n++){try{if((await fetch(origin+'/healthz')).ok)break;}catch{}if(n>150||server.exitCode!==null)throw new Error(logs);await new Promise(r=>setTimeout(r,100));}
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 // localhost is a secure context; no certificate-error bypass or security flags.
 const context=await browser.newContext();const page=await context.newPage();page.setDefaultTimeout(20000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Administration',exact:true}).waitFor();
 const fixture=await page.evaluate(async()=>{
  const call=async(method,path,value)=>{const response=await fetch('/api/v1'+path,{method,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},...(value===undefined?{}:{body:JSON.stringify(value)})});if(!response.ok)throw new Error(`${path}: ${response.status} ${await response.text()}`);return response.status===204?null:response.json();};
  const me=await call('GET','/me');const collection=await call('POST','/admin/collections',{name:'Local playback fixture',module:'org.junk.music'});const id=crypto.randomUUID();
  const item=await call('PUT','/items/'+id,{expected_revision:0,command:{collection_id:collection.id,title:'Private route audio',subtitle:'',identity_link:{kind:'none'},domain:{module:'org.junk.music',schema_version:1,payload:{}},identity:'unidentified',translations:[],identifiers:[],wishlist:false}});
  const length=44100*2*2*45;const wav=new Uint8Array(44+length);const view=new DataView(wav.buffer);const text=(offset,value)=>{for(let i=0;i<value.length;i++)wav[offset+i]=value.charCodeAt(i);};text(0,'RIFF');view.setUint32(4,length+36,true);text(8,'WAVEfmt ');view.setUint32(16,16,true);view.setUint16(20,1,true);view.setUint16(22,2,true);view.setUint32(24,44100,true);view.setUint32(28,176400,true);view.setUint16(32,4,true);view.setUint16(34,16,true);text(36,'data');view.setUint32(40,length,true);
  for(let frame=0;frame<length/4;frame++){const sample=Math.round(Math.sin(frame*2*Math.PI*440/44100)*1000);view.setInt16(44+frame*4,sample,true);view.setInt16(46+frame*4,sample,true);}
  const upload=await call('POST','/uploads',{item_id:id,expected_revision:item.revision,files:[{name:'tone.wav',bytes:wav.length,sha256:null}]});
  for(let offset=0;offset<wav.length;offset+=1024*1024){const response=await fetch(`/api/v1/uploads/${upload.id}/files/0?offset=${offset}`,{method:'PUT',headers:{'content-type':'application/octet-stream'},body:wav.slice(offset,offset+1024*1024)});if(!response.ok)throw new Error('fixture upload: '+await response.text());}
  await call('POST',`/uploads/${upload.id}/commit`);
  for(let n=0;n<100;n++){const current=await call('GET','/items/'+id);const rep=current.captures.flatMap(c=>c.representations)[0];if(rep){const track={id:crypto.randomUUID(),itemId:id,collectionId:collection.id,album:'Local fixture',title:'Private route audio',position:1,representationId:rep.id,fileIndex:0,mediaType:'audio/wav'};localStorage.setItem('collector.music-queue.v1.'+me.user_id,JSON.stringify({tracks:[track],current:track.id}));return {track,user:me.user_id,path:`/api/v1/items/${id}/representations/${rep.id}/0`};}await new Promise(r=>setTimeout(r,100));}throw new Error('fixture publication timed out');
 });
 await page.reload();const player=page.getByRole('region',{name:'Music player'});await player.waitFor();
 // Browsing never initiates QUIC or prompts; Play starts trusted discovery.
 assert.equal(await page.locator('audio').getAttribute('src'),null);
 await player.getByRole('button',{name:'Play',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentTime>.1;});
 assert.match(await player.innerText(),/Direct playback/);assert.equal(await page.evaluate(()=>!!navigator.serviceWorker.controller),true);
 const direct=await page.evaluate(async path=>(await fetch(path+'?control-negative=1',{method:'HEAD'})).status,fixture.path);
 // Query intentionally bypasses neither the worker nor server; use an unowned API client.
 const cookie=(await context.cookies()).map(c=>`${c.name}=${c.value}`).join('; ');const publicMedia=await fetch(origin+fixture.path,{headers:{cookie}});assert.equal(publicMedia.status,503,'control listener served public media');
 const slider=player.getByRole('slider',{name:'Seek in Private route audio'});await slider.fill('30');await page.waitForFunction(()=>document.querySelector('audio').currentTime>=29);await slider.fill('8');await page.waitForFunction(()=>{const n=document.querySelector('audio').currentTime;return n>=7&&n<12;});
 assert.equal(direct,200,'worker failed to own media requests');
 // Wrong pin must fail under the browser's ordinary security policy.
 assert.equal(await page.evaluate(async()=>{const descriptor=await(await fetch('/api/v1/playback/routes')).json();const connection=new WebTransport(descriptor.endpoints[0].webtransport_url,{serverCertificateHashes:[{algorithm:'sha-256',value:new Uint8Array(32)}]});try{await Promise.race([connection.ready,new Promise((_,reject)=>setTimeout(()=>reject(new Error('timeout')),3000))]);return false;}catch{return true;}finally{connection.close();void connection.closed.catch(()=>{});}}),true);
 // Server-side logout must stop even already buffered WebTransport playback.
 const revokedAudio=await page.locator('audio').elementHandle();
 assert.equal(await page.evaluate(async()=>(await fetch('/api/v1/auth/logout',{method:'POST'})).status),204);
 await page.getByText('Your session expired or was revoked.',{exact:false}).waitFor();
 assert.deepEqual(await revokedAudio.evaluate(audio=>({paused:audio.paused,src:audio.getAttribute('src')})),{paused:true,src:null});await revokedAudio.dispose();
 await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await player.waitFor();
 // Fail closed when the transport is unavailable: no ordinary HTTP media request.
 await page.reload();let mediaRequests=0;page.on('request',request=>{if(request.url().startsWith(origin+fixture.path))mediaRequests++;});
 await page.evaluate(()=>{window.WebTransport=undefined;});await player.getByRole('button',{name:'Play',exact:true}).click();await player.getByRole('alert').filter({hasText:'does not support secure local playback'}).waitFor();assert.equal(await page.locator('audio').getAttribute('src'),null);assert.equal(mediaRequests,0);
 assert.deepEqual(errors,[]);console.log(`Local playback passed in Chrome ${await browser.version()}: automatic pinned QUIC, actual audio, forward/back seeks, public media refusal, wrong pin, live logout/source release and unsupported-transport fail closed.`);
}finally{
 await browser?.close();server.kill('SIGTERM');await Promise.race([new Promise(r=>server.once('exit',r)),new Promise(r=>setTimeout(r,4000))]);if(server.exitCode===null){server.kill('SIGKILL');await new Promise(r=>server.once('exit',r));}await rm(scratch,{recursive:true,force:true});
}
