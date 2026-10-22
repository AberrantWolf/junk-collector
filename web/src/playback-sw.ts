// The worker streams bounded ranges through the owning, authenticated document.
// No cookies, private keys, endpoint probing or implicit HTTP fallback live here.
export {};
interface MediaReply {status:number;headers:Record<string,string>;body?:ArrayBuffer;direct?:boolean;error?:string}
interface WorkerClient {postMessage(value:unknown,transfer:Transferable[]):void}
interface WorkerScope {clients:{claim():Promise<void>;get(id:string):Promise<WorkerClient|undefined>};skipWaiting():Promise<void>;addEventListener(type:string,handler:(event:any)=>void):void}
const worker=self as unknown as WorkerScope;
worker.addEventListener('install',event=>event.waitUntil(worker.skipWaiting()));
worker.addEventListener('activate',event=>event.waitUntil(worker.clients.claim()));
function rpc(client:WorkerClient,url:string,method:string,start?:number,end?:number,etag?:string,signal?:AbortSignal):Promise<MediaReply>{
 return new Promise((resolve,reject)=>{const channel=new MessageChannel();const timer=setTimeout(()=>finish(new Error('Local playback timed out. Use Play to retry.')),15000);
 const abort=()=>{channel.port1.postMessage({cancel:true});finish(new DOMException('Cancelled','AbortError'));};
 function finish(error?:Error,result?:MediaReply){clearTimeout(timer);signal?.removeEventListener('abort',abort);channel.port1.close();error?reject(error):resolve(result!);}
 channel.port1.onmessage=event=>finish(undefined,event.data as MediaReply);
 signal?.addEventListener('abort',abort,{once:true});if(signal?.aborted){abort();return;}
 client.postMessage({type:'collector-media-read',url,method,start,end,etag},[channel.port2]);
 });
}
worker.addEventListener('fetch',event=>{
 const request=event.request as Request;const url=new URL(request.url);
 if(url.origin!==self.location.origin||!/^\/api\/v1\/items\/[0-9a-f-]+\/representations\/[0-9a-f-]+\/\d+$/.test(url.pathname)||!['GET','HEAD'].includes(request.method))return;
 event.respondWith((async()=>{
  try{
   const client=await worker.clients.get(event.clientId);if(!client)throw new Error('Open Collector to play local media.');
   const metadata=await rpc(client,url.pathname,'HEAD',undefined,undefined,undefined,request.signal);
   if(metadata.direct)return fetch(request);
   if(metadata.status!==200)return new Response(request.method==='HEAD'?null:metadata.body,{status:metadata.status,headers:metadata.headers});
   const headers=new Headers(metadata.headers);headers.set('Cache-Control','private, no-store');
   if(request.method==='HEAD')return new Response(null,{status:200,headers});
   const length=Number(headers.get('content-length'));const etag=headers.get('etag');if(!Number.isSafeInteger(length)||length<0||!etag)throw new Error('Invalid local media metadata');
   const range=request.headers.get('range');let start=0,end=length-1;
   if(range){const match=/^bytes=(\d*)-(\d*)$/.exec(range);if(!match||(!match[1]&&!match[2]))return new Response(null,{status:416});
    if(!match[1])start=Math.max(0,length-Number(match[2]));else start=Number(match[1]);if(match[1]&&match[2])end=Math.min(end,Number(match[2]));
    if(!Number.isSafeInteger(start)||!Number.isSafeInteger(end)||start<0||start>end||start>=length)return new Response(null,{status:416,headers:{'Content-Range':`bytes */${length}`}});
   }
   if(request.headers.has('if-range')&&request.headers.get('if-range')!==etag)return new Response(null,{status:409});
   const controller=new AbortController();request.signal.addEventListener('abort',()=>controller.abort(),{once:true});
   let position=start;
   let first:MediaReply|undefined=await rpc(client,url.pathname,'GET',position,Math.min(end,position+256*1024-1),etag,controller.signal);
   if(first.status!==206)return new Response(first.body,{status:first.status,headers:first.headers});
   const body=new ReadableStream<Uint8Array>({async pull(stream){try{
    if(position>end){stream.close();return;}
    const last=Math.min(end,position+256*1024-1);const reply=first??await rpc(client,url.pathname,'GET',position,last,etag,controller.signal);first=undefined;
    if(reply.status!==206||reply.headers.etag!==etag||reply.headers['content-range']!==`bytes ${position}-${last}/${length}`||!(reply.body instanceof ArrayBuffer)||reply.body.byteLength!==last-position+1)throw new Error('Local media access failed or changed.');
    position=last+1;stream.enqueue(new Uint8Array(reply.body));
   }catch(error){controller.abort();stream.error(error);}},cancel(){controller.abort();}}, {highWaterMark:1});
   headers.set('Content-Length',String(end-start+1));if(range)headers.set('Content-Range',`bytes ${start}-${end}/${length}`);
   return new Response(body,{status:range?206:200,headers});
  }catch{return new Response('Local playback unavailable. Public media fallback is disabled.',{status:503,headers:{'Content-Type':'text/plain','Cache-Control':'no-store'}});}
 })());
});
