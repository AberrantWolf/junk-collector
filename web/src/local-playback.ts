import {api,getApiIdentity,ApiRequestError} from './runtime';
import type {components} from './generated/api';
type Descriptor=components['schemas']['LocalPlaybackDescriptor'];
type Target=components['schemas']['PlaybackTarget'];
type Grant=components['schemas']['BrowserPlaybackGrant'];
interface Transport {ready:Promise<void>;closed:Promise<unknown>;close():void;createBidirectionalStream():Promise<{readable:ReadableStream<Uint8Array>;writable:WritableStream<Uint8Array>}>}
interface TransportConstructor {new(url:string,options:{serverCertificateHashes:{algorithm:string;value:Uint8Array}[]}):Transport}
const constructor=()=>(globalThis as unknown as {WebTransport?:TransportConstructor}).WebTransport;
let route:Transport|undefined;let descriptor:Descriptor|undefined;let refreshed=0;let epoch=0;let selecting:Promise<void>|undefined;let registration:Promise<void>|undefined;
const grants=new Map<string,Grant>();
export let playbackRoute='Checking local playback';
function reset(){epoch++;route?.close();route=undefined;descriptor=undefined;refreshed=0;grants.clear();selecting=undefined;playbackRoute='Checking local playback';}
window.addEventListener('collector-api-identity-changed',reset);
window.addEventListener('online',reset);
(navigator as unknown as {connection?:EventTarget}).connection?.addEventListener('change',reset);
function privateHost(host:string){host=host.replace(/^\[|\]$/g,'');if(host==='::1'||/^f[cd][0-9a-f]*:/i.test(host))return true;const n=host.split('.').map(Number);return n.length===4&&n.every(x=>Number.isInteger(x)&&x>=0&&x<=255)&&(n[0]===10||n[0]===127||n[0]===192&&n[1]===168||n[0]===172&&n[1]>=16&&n[1]<=31||n[0]===100&&n[1]>=64&&n[1]<=127);}
async function bounded<T>(promise:Promise<T>,milliseconds:number,message:string):Promise<T>{let timer:ReturnType<typeof setTimeout>;try{return await Promise.race([promise,new Promise<never>((_,reject)=>{timer=setTimeout(()=>reject(new Error(message)),milliseconds);})]);}finally{clearTimeout(timer!);}}
async function select(){
 const token=epoch;const identity=getApiIdentity();if(!identity)throw new Error('Sign in to use local playback.');
 const [advertised,server]=await Promise.all([api<Descriptor>('/playback/routes'),api<{server_id:string}>('/identity')]);
 if(token!==epoch)throw new Error('Playback account changed.');
 if(advertised.server_id!==server.server_id||advertised.endpoints.length>8)throw new Error('Playback server identity mismatch.');
 descriptor=advertised;refreshed=Date.now();
 let permission:PermissionState|undefined;
 for(const name of ['local-network','local-network-access']){try{permission=(await navigator.permissions.query({name:name as PermissionName})).state;break;}catch{}}
 if(permission==='denied'&&!advertised.remote_media_allowed)throw new Error('Local network access is blocked for Collector. Allow it in this site’s permissions, then use Play to retry. Public media fallback is disabled.');
 const WT=constructor();
 if(WT&&advertised.endpoints.length){
  // The first verified candidate wins. No credential is sent during the handshake.
  const candidates=advertised.endpoints.map(async endpoint=>{
   const url=new URL(endpoint.webtransport_url);if(url.protocol!=='https:'||!privateHost(url.hostname)||url.username||url.password||url.pathname!=='/playback'||url.search||url.hash||!/^([0-9a-f]{2}){32}$/.test(endpoint.certificate_sha256)||Date.parse(endpoint.expires_at)<=Date.now()||Date.parse(endpoint.expires_at)>Date.now()+8*86400000)throw new Error('Invalid local playback endpoint.');
   const pin=Uint8Array.from(endpoint.certificate_sha256.match(/../g)!,b=>parseInt(b,16));
   const connection=new WT(url.href,{serverCertificateHashes:[{algorithm:'sha-256',value:pin}]});void connection.closed.catch(()=>{});
   try{await bounded(connection.ready,permission==='prompt'?60000:2500,'Local connection timed out.');if(token!==epoch)throw new Error('Playback account changed.');return connection;}catch(error){connection.close();throw error;}
  });
  try{
   const winner=await Promise.any(candidates);for(const candidate of candidates)void candidate.then(connection=>{if(connection!==winner)connection.close();},()=>{});
   if(token!==epoch){winner.close();throw new Error('Playback account changed.');}
   route?.close();route=winner;playbackRoute='Direct playback';void winner.closed.finally(()=>{if(route===winner){route=undefined;refreshed=0;}}).catch(()=>{});return;
  }catch{if(!advertised.remote_media_allowed)throw new Error('No trusted local playback route is reachable. Allow Local network access for this site, then use Play to retry. Check that the server allows local UDP playback. Public media fallback is disabled.');}
 }
 if(advertised.remote_media_allowed){playbackRoute='HTTPS playback';return;}
 throw new Error(WT?'No local playback endpoint is available. Public media fallback is disabled.':'This browser does not support secure local playback. Use a browser with WebTransport support, or the Trash Amp app. Public media fallback is disabled.');
}
async function ensureRoute(){if(route||descriptor&&descriptor.remote_media_allowed&&Date.now()-refreshed<30000)return;if(!selecting){const pending=select();selecting=pending;void pending.finally(()=>{if(selecting===pending)selecting=undefined;}).catch(()=>{});}return selecting;}
async function install(){if(!('serviceWorker'in navigator))throw new Error('Local playback needs service worker support in a secure browser.');await navigator.serviceWorker.register('/playback-sw.js',{scope:'/',updateViaCache:'none'});await navigator.serviceWorker.ready;if(!navigator.serviceWorker.controller)await bounded(new Promise<void>(resolve=>navigator.serviceWorker.addEventListener('controllerchange',()=>resolve(),{once:true})),5000,'Reload Collector to finish enabling local playback.');}
export async function prepareLocalPlayback(signal:AbortSignal){signal.throwIfAborted();await ensureRoute();signal.throwIfAborted();if(route){registration??=install().catch(error=>{registration=undefined;throw error;});await registration;}signal.throwIfAborted();return playbackRoute;}
function targetFromPath(path:string):Target {const parts=/^\/api\/v1\/items\/([0-9a-f-]+)\/representations\/([0-9a-f-]+)\/(\d+)$/.exec(path);if(!parts)throw new Error('Invalid media target');return {item_id:parts[1],representation_id:parts[2],file_index:Number(parts[3])};}
async function read(path:string,method:string,start?:number,end?:number,etag?:string,signal?:AbortSignal){
 const submitted=epoch;
 try{return await readOnce(path,method,start,end,etag,signal);}catch(error){
  if(error instanceof ApiRequestError||signal?.aborted||submitted!==epoch)throw error;
  route?.close();route=undefined;descriptor=undefined;refreshed=0;
  return readOnce(path,method,start,end,etag,signal);
 }
}
async function readOnce(path:string,method:string,start?:number,end?:number,etag?:string,signal?:AbortSignal){
 const token=epoch;const identity=getApiIdentity()?.user_id;await ensureRoute();if(token!==epoch||identity!==getApiIdentity()?.user_id)throw new Error('Playback account changed.');
 if(!route){if(descriptor?.remote_media_allowed)return {direct:true,status:200,headers:{}};throw new Error('Local playback disconnected.');}
 const connection=route;
 let grant=grants.get(path);if(!grant||Date.parse(grant.expires_at)-Date.now()<15000){grant=await api<Grant>('/playback/grants',{method:'POST',body:JSON.stringify(targetFromPath(path))});if(token!==epoch)throw new Error('Playback account changed.');if(grants.size>=8)grants.delete(grants.keys().next().value!);grants.set(path,grant);}
 const stream=await connection.createBidirectionalStream();const writer=stream.writable.getWriter();const reader=stream.readable.getReader();
 const abort=()=>{void reader.cancel().catch(()=>{});void writer.abort().catch(()=>{});};signal?.addEventListener('abort',abort,{once:true});
 try{
  signal?.throwIfAborted();
  await writer.write(new TextEncoder().encode(JSON.stringify({token:grant.token,target:targetFromPath(path),method,start:start??null,end:end??null,etag:etag??null})));await writer.close();
  const chunks:Uint8Array[]=[];let length=0;
  while(true){const part=await reader.read();if(part.done)break;length+=part.value.length;if(length>256*1024+8192)throw new Error('Oversized media response');chunks.push(part.value);}
  if(token!==epoch||identity!==getApiIdentity()?.user_id)throw new Error('Playback account changed.');
  const bytes=new Uint8Array(length);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
  const newline=bytes.indexOf(10);if(newline<0||newline>8192)throw new Error('Invalid media response');
  const reply=JSON.parse(new TextDecoder().decode(bytes.subarray(0,newline))) as {status:number;headers:Record<string,string>};
  if(reply.status===401||reply.status===403){grants.delete(path);throw new ApiRequestError(reply.status,'media_access_denied','Access to this queued album is no longer available.');}
  return {...reply,body:bytes.slice(newline+1).buffer};
 }finally{signal?.removeEventListener('abort',abort);void reader.cancel().catch(()=>{});reader.releaseLock();writer.releaseLock();}
}
if('serviceWorker'in navigator)navigator.serviceWorker.addEventListener('message',event=>{
 const message=event.data;if(message?.type!=='collector-media-read'||!event.ports[0])return;const port=event.ports[0];const controller=new AbortController();port.onmessage=e=>{if(e.data?.cancel)controller.abort();};
 void bounded(read(message.url,message.method,message.start,message.end,message.etag,controller.signal),12000,'Local playback timed out.').then(reply=>{if('body'in reply&&reply.body instanceof ArrayBuffer)port.postMessage(reply,[reply.body]);else port.postMessage(reply);},error=>port.postMessage({status:error instanceof ApiRequestError?error.status:503,headers:{},error:String(error)})).finally(()=>{controller.abort();port.close();});
});

// Permission-free control check: polling must not trigger a local-network prompt.
export async function checkPlaybackAccess(path:string,signal:AbortSignal){
 const grant=await api<Grant>('/playback/grants',{method:'POST',body:JSON.stringify(targetFromPath(path)),signal});
 if(grants.size>=8)grants.delete(grants.keys().next().value!);grants.set(path,grant);
}
