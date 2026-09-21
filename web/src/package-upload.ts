import {ApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';
type ImportView=components['schemas']['ImportView'];
export type UploadProgress={phase:'checking'|'uploading';file:string;index:number;count:number;completed:number;total:number;confirmed:number;packageBytes:number;waiting:boolean};
const chunkBytes=4*1024*1024;
function fileName(file:File){return file.webkitRelativePath||file.name;}

// XHR exposes actual request-body progress; fetch only reports the final response.
// Chunk offsets make one network retry safe, including a lost acknowledgement.
async function sendChunk(path:string,body:Blob,signal:AbortSignal,progress:(loaded:number,waiting:boolean)=>void):Promise<ImportView>{
 const key=crypto.randomUUID();
 for(let attempt=0;;attempt++){
  signal.throwIfAborted();
  try{return await new Promise<ImportView>((resolve,reject)=>{
   const xhr=new XMLHttpRequest();xhr.open('PUT',`/api/v1${path}`);xhr.withCredentials=true;xhr.setRequestHeader('Idempotency-Key',key);
   const cleanup=()=>signal.removeEventListener('abort',abort);
   const abort=()=>{xhr.abort();cleanup();reject(new DOMException('Upload stopped','AbortError'));};
   signal.addEventListener('abort',abort,{once:true});
   xhr.upload.onprogress=event=>{if(!signal.aborted)progress(Math.min(event.loaded,body.size),false);};
   xhr.upload.onload=()=>{if(!signal.aborted)progress(body.size,true);};
   xhr.onerror=()=>{cleanup();reject(new TypeError('Upload connection interrupted. Reselect the package to resume.'));};
   xhr.onabort=()=>{cleanup();reject(new DOMException('Upload stopped','AbortError'));};
   xhr.onload=()=>{
    cleanup();if(signal.aborted){reject(new DOMException('Upload stopped','AbortError'));return;}
    let value;try{value=JSON.parse(xhr.responseText);}catch{reject(new Error('The server returned an unreadable upload response.'));return;}
    if(xhr.status<200||xhr.status>=300){reject(new ApiRequestError(xhr.status,value.error?.code||`http_${xhr.status}`,value.error?.message||`Upload failed (${xhr.status})`));return;}
    resolve(value as ImportView);
   };
   progress(0,false);if(signal.aborted){abort();return;}xhr.send(body);
  });}catch(error){if(signal.aborted||attempt>0||!(error instanceof TypeError))throw error;}
 }
}

export async function uploadPackage(initial:ImportView,files:File[],verifyPrefix:boolean,signal:AbortSignal,onProgress:(value:UploadProgress)=>void,onRecord:(value:ImportView)=>void):Promise<ImportView>{
 const selected=new Map(files.map(file=>[fileName(file),file]));
 if(selected.size!==files.length||selected.size!==initial.files.length||initial.files.some(entry=>selected.get(entry.name)?.size!==entry.bytes))throw new Error('Select the same complete package, with original file names and sizes.');
 let current=initial;
 const packageBytes=initial.files.reduce((sum,file)=>sum+file.bytes,0);
 const retained=initial.files.reduce((sum,file)=>sum+file.received_bytes,0);
 let checked=0;
 async function transfer(index:number,offset:number,end:number,phase:UploadProgress['phase']){
  const entry=initial.files[index];const file=selected.get(entry.name)!;
  const confirmed=current.files.reduce((sum,value)=>sum+value.received_bytes,0);
  const base=phase==='checking'?checked:confirmed;
  current=await sendChunk(`/imports/${initial.id}/files/${index}?offset=${offset}`,file.slice(offset,end),signal,(loaded,waiting)=>onProgress({phase,file:entry.name,index:index+1,count:initial.files.length,completed:base+loaded,total:phase==='checking'?retained:packageBytes,confirmed,packageBytes,waiting}));
  signal.throwIfAborted();onRecord(current);
 }
 // Verify every retained prefix before appending any file, preserving resume safety.
 if(verifyPrefix)for(let index=0;index<initial.files.length;index++){
  const entry=initial.files[index];
  for(let offset=0;offset<entry.received_bytes;offset+=chunkBytes){const end=Math.min(offset+chunkBytes,entry.received_bytes);await transfer(index,offset,end,'checking');checked+=end-offset;}
 }
 for(let index=0;index<initial.files.length;index++){
  const entry=initial.files[index];
  for(let offset=current.files[index].received_bytes;offset<entry.bytes;offset+=chunkBytes)await transfer(index,offset,Math.min(offset+chunkBytes,entry.bytes),'uploading');
 }
 return current;
}
