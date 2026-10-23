import React,{useEffect,useRef,useState,api,requestErrorMessage} from './runtime';
import {useMusicQueue} from './music-queue';
import type {components} from './generated/api';
type Test=components['schemas']['RemoteMediaTest'];
type Descriptor=components['schemas']['LocalPlaybackDescriptor'];
const digest=async(bytes:ArrayBuffer)=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),n=>n.toString(16).padStart(2,'0')).join('');

export default function RemoteMediaTest(){
 const queue=useMusicQueue();const track=queue.tracks.find(t=>t.id===queue.current);
 const [cellular,setCellular]=useState(false);const [busy,setBusy]=useState(false);const [error,setError]=useState('');const [results,setResults]=useState<string[]>([]);const active=useRef<AbortController|undefined>(undefined);
 useEffect(()=>()=>{active.current?.abort();active.current=undefined;},[]);
 async function run(){
  active.current?.abort();const controller=new AbortController();active.current=controller;setBusy(true);setError('');setResults([]);
  const stages:string[]=[];const stage=(message:string)=>{controller.signal.throwIfAborted();stages.push(message);setResults([...stages]);};
  try{
   const routes=await api<Descriptor>('/playback/routes',{signal:controller.signal});const endpoint=routes.remote_media;
   if(!endpoint)throw new Error('Direct HTTPS media is not configured. Ask the server owner to finish setup.');
   const base=new URL(endpoint.base_url);if(base.protocol!=='https:'||base.username||base.password||base.search||base.hash)throw new Error('Invalid media endpoint.');
   const get=async(test:Test,range?:string)=>{
    const url=new URL(test.grant.url);if(url.username||url.password||url.hash||url.origin!==base.origin||url.pathname!==new URL('read',base).pathname||test.grant.server_id!==routes.server_id)throw new Error('Media authority changed.');
    const response=await fetch(url,{credentials:'omit',redirect:'error',cache:'no-store',signal:AbortSignal.any([controller.signal,AbortSignal.timeout(10000)]),headers:range?{Range:range}:{}});
    if(response.status!==(range?206:200)||response.headers.get('etag')!==test.grant.etag)throw new Error('Media response or immutable validator did not match.');
    const wanted=range?Number(range.split('-')[1])-Number(range.split('=')[1].split('-')[0])+1:test.grant.length;
    if(!Number.isSafeInteger(wanted)||wanted<1||wanted>65536||Number(response.headers.get('content-length'))!==wanted)throw new Error('Unexpected media probe length.');
    if(range&&response.headers.get('content-range')!==`bytes ${range.slice(6)}/${test.grant.length}`)throw new Error('Media range did not match.');
    const reader=response.body!.getReader();const bytes=new Uint8Array(wanted);let length=0;
    try{while(true){const result=await reader.read();if(result.done)break;if(result.value.length>wanted-length)throw new Error('Oversized probe response.');bytes.set(result.value,length);length+=result.value.length;}}finally{await reader.cancel().catch(()=>{});}
    if(length!==wanted)throw new Error('Truncated probe response.');return bytes;
   };
   const probe=await api<Test>('/playback/remote-test',{method:'POST',body:JSON.stringify({target:null}),signal:controller.signal});
   const bytes=await get(probe);const expected=`collector-media-probe-v1:${routes.server_id}:${await digest(new TextEncoder().encode(probe.grant.token).buffer)}`;
   if(new TextDecoder().decode(bytes)!==expected)throw new Error('Fresh media challenge did not match.');
   stage(`HTTPS transport confirmed at ${new Date().toLocaleString()}. Observer: ${cellular?'user-confirmed cellular with Wi-Fi off':'current network; external reachability unproven'}.`);
   if(track){
    const test=await api<Test>('/playback/remote-test',{method:'POST',body:JSON.stringify({target:{item_id:track.itemId,representation_id:track.representationId,file_index:track.fileIndex}}),signal:controller.signal});
    for(const sample of test.samples){const bytes=await get(test,`bytes=${sample.start}-${sample.end}`);if(await digest(bytes.buffer)!==sample.digest.hex)throw new Error('Media bytes did not match the authorized source.');}
    stage(`Authenticated early and late media reads confirmed for ${track.title}.`);
   }else stage('Real media read untested: add a track to your queue, then repeat this test.');
   stage('Playback is a separate check: return to the library, press Play, seek forward/back, then test screen lock and next track. This page does not certify audio output or security.');
  }catch(error){if(!controller.signal.aborted)setError(`${requestErrorMessage(error)} A browser network error can mean DNS, connection, TLS or CORS failure; this page cannot reliably distinguish them.`);}finally{if(active.current===controller)setBusy(false);}
 }
 return <main className="shell"><section className="detail-content" aria-label="Remote media test"><h1>Test remote playback access</h1><p>On your iPhone, turn Wi-Fi off and use cellular data to test access from outside home. The test uses a fresh small challenge and, when a track is queued, two tiny authorized audio ranges.</p><label><input type="checkbox" checked={cellular} disabled={busy} onChange={e=>setCellular(e.target.checked)}/>I have turned Wi-Fi off and am using cellular data</label><p>The browser cannot independently verify your network location. A test from home can succeed through router loopback without proving remote access.</p><button disabled={busy} onClick={()=>void run()}>{busy?'Checking…':'Test remote access'}</button>{busy&&<button onClick={()=>{active.current?.abort();setBusy(false);}}>Cancel</button>}{results.length>0&&<ol aria-live="polite">{results.map((result,index)=><li key={index}>{result}</li>)}</ol>}{error&&<p role="alert">{error}</p>}<p>Results apply to this endpoint at this time. They do not change server settings or open firewall ports.</p><a href="/">Back to your library</a></section></main>;
}
