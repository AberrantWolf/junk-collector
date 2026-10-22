import React, {useEffect,useState} from '../../../../web/src/runtime';
import {createRoot} from 'react-dom/client';
import MusicPlayer from '../../../../web/src/music-player';
import {setMusicQueueUser,startMusicTracks} from '../../../../web/src/music-queue';

type Reply={status:number;headers:Record<string,string>;body:ArrayBuffer};
let config:any, transport:any, lease='', leaseUntil=0;
const base=new URL('./',location.href);
const control=(path:string)=>new URL(path.replace(/^\//,''),base).toString();
const networkPermissions:Record<string,string>={};
const encoder=new TextEncoder(), decoder=new TextDecoder();
const events:string[]=[];
function record(message:string){events.push(`${new Date().toISOString().slice(11,19)} ${message}`);if(events.length>50)events.shift();}
async function bounded<T>(promise:Promise<T>, ms=12000):Promise<T>{let timer:any;try{return await Promise.race([promise,new Promise<T>((_,reject)=>timer=setTimeout(()=>reject(Error('Operation timed out')),ms))]);}finally{clearTimeout(timer);}}
async function connect(hash=config.hash, timeout=60000){
 const wt=new (window as any).WebTransport(config.endpoint,{serverCertificateHashes:[{algorithm:'sha-256',value:new Uint8Array(hash)}]});
 wt.closed.catch(()=>{});
 try{await bounded(wt.ready,timeout);return wt;}catch(e){wt.close();throw e;}
}
function validateDescriptor(value:any){
 if(value?.server_id!==config.server_id||value.origin!==location.origin||value.path!==config.path
    ||value.endpoint!==config.endpoint||!Array.isArray(value.hash)||value.hash.length!==32
    ||!value.hash.every((v:any)=>Number.isInteger(v)&&v>=0&&v<=255))throw Error('Unbound transport descriptor rejected');
}
async function grant(){const response=await fetch(control('/grant'),{method:'POST'});if(!response.ok)throw Error(`Grant HTTP ${response.status}`);const value=await response.json();validateDescriptor(value.descriptor);config={...config,...value.descriptor};lease=value.token;leaseUntil=Date.now()+value.expires_seconds*1000-5000;return value;}
async function rpc(value:any, token=lease):Promise<Reply>{
 if(!transport)throw Error('Connect WebTransport first');
 const stream=await bounded(transport.createBidirectionalStream());
 const writer=stream.writable.getWriter(), reader=stream.readable.getReader();
 try{
  await writer.write(encoder.encode(JSON.stringify({...value,token})));await writer.close();
  const chunks:Uint8Array[]=[];let count=0;
  while(true){const {done,value:chunk}=await bounded(reader.read());if(done)break;count+=chunk.length;if(count>270336)throw Error('Response bound');chunks.push(chunk);}
  const bytes=new Uint8Array(count);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
  const boundary=bytes.indexOf(10);if(boundary<0||boundary>8192)throw Error('Bad response header');
  const header=JSON.parse(decoder.decode(bytes.subarray(0,boundary)));
  return {...header,body:bytes.slice(boundary+1).buffer};
 }finally{reader.cancel().catch(()=>{});reader.releaseLock();writer.releaseLock();}
}
navigator.serviceWorker.addEventListener('message',event=>{
 if(event.data?.type==='adapter-event'){record(event.data.value);return;}
 if(event.data?.type!=='wt-read'||!event.ports[0])return;
 const port=event.ports[0];
 void (async()=>{try{if(Date.now()>leaseUntil)await grant();const result=await rpc(event.data.value);if(event.data.value.method==='GET')record(`WT ${event.data.value.range}: HTTP ${result.status}, ${result.body.byteLength} bytes`);port.postMessage(result,[result.body]);}catch(e){record('Media adapter: '+String(e));port.postMessage({error:String(e)});}finally{port.close();}})();
});

function App(){
 const [ready,setReady]=useState(false),[username,setUsername]=useState('listener'),[password,setPassword]=useState(''),[message,setMessage]=useState('Preparing service worker…'),[snapshot,setSnapshot]=useState(''),[busy,setBusy]=useState(false),[checks,setChecks]=useState<any>(null);
 async function action(task:()=>Promise<void>){setBusy(true);try{await task();}catch(e){setMessage(String(e));record(String(e));}finally{setBusy(false);}}
 useEffect(()=>{void action(async()=>{
  config=await(await fetch(control('/fixture'))).json();
  if(config.origin!==location.origin)throw Error('Unexpected control origin');
  for(const name of ['local-network','loopback-network','local-network-access']){
   try{const permission=await navigator.permissions.query({name} as any);networkPermissions[name]=permission.state;permission.addEventListener('change',()=>{networkPermissions[name]=permission.state;record(`${name} permission: ${permission.state}`);});}catch{networkPermissions[name]='not supported';}
  }
  document.addEventListener('securitypolicyviolation',event=>record(`CSP blocked ${event.effectiveDirective}: ${event.blockedURI}`));
  // Chromium's service-worker media path requires CORS mode even for same-origin audio.
  const audio=document.querySelector('audio');if(audio)audio.crossOrigin='anonymous';
  const script=control('/sw.js');
  await navigator.serviceWorker.register(script,{scope:base.pathname});
  if(navigator.serviceWorker.controller?.scriptURL!==script)await bounded(new Promise<void>(resolve=>{const changed=()=>{if(navigator.serviceWorker.controller?.scriptURL===script){navigator.serviceWorker.removeEventListener('controllerchange',changed);resolve();}};navigator.serviceWorker.addEventListener('controllerchange',changed);changed();}));
  setMessage('Ready. Sign in with the disposable test credentials.');setReady(true);
 });const timer=setInterval(()=>{const audio=document.querySelector('audio');setSnapshot(JSON.stringify({origin:location.origin,secureContext:isSecureContext,viaCloudflare:config?.via_cloudflare,worker:navigator.serviceWorker.controller?.scriptURL,permissions:networkPermissions,webTransport:!!transport,tokenLifetimeSeconds:Math.max(0,Math.round((leaseUntil-Date.now())/1000)),audio:audio?{paused:audio.paused,time:Math.round(audio.currentTime*100)/100,duration:audio.duration,readyState:audio.readyState,srcAttached:audio.hasAttribute('src'),buffered:Array.from({length:audio.buffered.length},(_,i)=>[audio.buffered.start(i),audio.buffered.end(i)])}:null,events},null,2));},1000);return()=>clearInterval(timer);},[]);
 async function login(){
  const response=await fetch(control('/api/v1/auth/login'),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({username,password,device_name:'Disposable WebTransport LAN test'})});
  if(!response.ok)throw Error(`Sign-in HTTP ${response.status}`);
  await grant();transport?.close();transport=await connect();
  setMusicQueueUser('wt-experiment-'+username);record(`Signed in as ${username}; pinned LAN transport ready`);setMessage('Connected. Play synthetic track uses the unchanged Collector MusicPlayer.');
 }
 async function negativeChecks(){
  const results:any={};
  try{validateDescriptor({...config,server_id:'unrelated'});results.wrongServer='FAIL accepted';}catch{results.wrongServer='PASS rejected';}
  try{validateDescriptor({...config,origin:'https://unrelated.invalid'});results.wrongOrigin='FAIL accepted';}catch{results.wrongOrigin='PASS rejected';}
  const bad=[...config.hash];bad[0]^=255;
  try{const wrong=await connect(bad,8000);wrong.close();results.wrongPin='FAIL connected';}catch{results.wrongPin='PASS rejected';}
  results.missingGrant=(await rpc({path:config.path,method:'HEAD'},'')).status;
  results.arbitraryPath=(await rpc({path:'/api/v1/admin/users',method:'HEAD'})).status;
  results.largeRange=(await rpc({path:config.path,method:'GET',range:'bytes=0-99999999'})).status;
  results.authorizedRead=(await rpc({path:config.path,method:'GET',range:'bytes=0-15'})).status;
  record('Authorization/pin controls '+JSON.stringify(results));setChecks(results);setMessage('Control checks completed. Rejections below are intentional tests, not warnings about the correct site.');
 }
 async function revoke(){
  const old=lease;const response=await fetch(control('/api/v1/auth/logout'),{method:'POST'});
  const check=await rpc({path:config.path,method:'HEAD'},old);
  record(`Logout HTTP ${response.status}; old grant against actual Collector HTTP ${check.status}`);
  setMessage(`Revoked: old grant returns ${check.status}. Watch the real player's availability poll pause and detach audio within 5 seconds.`);
 }
 async function expired(){const response=await rpc({path:config.path,method:'HEAD'},lease);record(`Current lease HEAD HTTP ${response.status}`);setMessage(`Current lease HEAD HTTP ${response.status}`);}
 async function report(){const stats=await(await fetch(control('/stats'))).json();const value={browser:navigator.userAgent,snapshot:JSON.parse(snapshot),checks,server:stats};await fetch(control('/result'),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(value)});setMessage('Diagnostic result saved on the NAS.');}
 return <><form onSubmit={e=>{e.preventDefault();void action(login);}}><label>Test username <input value={username} onChange={e=>setUsername(e.target.value)} autoComplete="username"/></label><label>Test password <input type="password" value={password} onChange={e=>setPassword(e.target.value)} autoComplete="current-password"/></label><button disabled={!ready||busy}>Sign in and connect</button></form>
 <p>Connecting needs permission for this site to reach your LAN. If blocked, use the browser’s site controls to allow Local network, then retry. A permission grant does not replace certificate or account checks.</p>
 <p role="status">{message}</p><p><button disabled={!transport||busy} onClick={()=>{startMusicTracks([config.track]);record('Queued real player');}}>Play synthetic track</button><button disabled={!transport||busy} onClick={()=>void action(negativeChecks)}>Run pin and authorization checks</button><button disabled={!transport||busy} onClick={()=>void action(revoke)}>Revoke this test session</button><button disabled={!transport||busy} onClick={()=>void action(expired)}>Check current lease</button><button onClick={()=>void action(report)}>Save diagnostics</button></p>
 <label><input type="checkbox" onChange={e=>{const audio=document.querySelector('audio');if(audio)audio.muted=e.target.checked;record('Mute '+e.target.checked);}}/> Mute synthetic audio (decoding still runs)</label>
 <MusicPlayer browsingMusic={true} onSessionExpired={()=>record('Real player reported session expired')}/>
 {checks&&<section aria-label="Control check results"><h2>Expected rejection checks</h2><p>The wrong server, origin and pin are deliberately supplied. PASS rejected means the protection worked. These results stay visible after saving diagnostics.</p><pre>{JSON.stringify(checks,null,2)}</pre></section>}
 <h2>Live diagnostics</h2><p>Use the real player’s seek slider. Test credentials work only in this disposable installation. “denied” has no access to the audio collection.</p><pre>{snapshot}</pre></>;
}
createRoot(document.getElementById('root')!).render(<App/>);
