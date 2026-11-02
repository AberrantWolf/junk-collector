import {api,getApiIdentity,isApiRequestError,requestErrorMessage} from './runtime';
export type DigitalSubmission={key:string;expected_revision:number;extended:boolean;refresh?:boolean;itemId:string;title:string;userId:string;error?:string;failure?:'rejected'|'uncertain'};
const prefix='collector.digital-match.';
const active=new Map<string,Promise<{job_id:string}>>();
export function pendingDigitalSubmissions(userId:string):DigitalSubmission[]{
 const values:DigitalSubmission[]=[];
 for(const key of Object.keys(localStorage).filter(key=>key.startsWith(`${prefix}${userId}.`)).slice(0,128)){
  try{const saved=JSON.parse(localStorage.getItem(key)||'null');if(saved?.key&&Number.isSafeInteger(saved.expected_revision))values.push({...saved,userId,itemId:key.slice(`${prefix}${userId}.`.length),title:saved.title||'Album',extended:!!saved.extended});}catch{}
 }
 return values;
}
function storage(value:DigitalSubmission){return `${prefix}${value.userId}.${value.itemId}`;}
function changed(value:DigitalSubmission,jobId?:string){window.dispatchEvent(new CustomEvent('collector-digital-submission',{detail:{...value,jobId}}));}
export function submitDigitalAlbum(value:DigitalSubmission):Promise<{job_id:string}>{
 if(getApiIdentity()?.user_id!==value.userId)return Promise.reject(new Error('Sign in to queue this album.'));
 const name=storage(value);const existing=active.get(name);if(existing)return existing;
 const frozen={...value,error:undefined,failure:undefined};localStorage.setItem(name,JSON.stringify(frozen));changed(frozen);
 // Admission belongs to the signed-in session, independently of album-panel lifetime.
 const attempt=api<{job_id:string}>(`/items/${value.itemId}/digital-album-matches`,{method:'POST',headers:{'Idempotency-Key':value.key},body:JSON.stringify({expected_revision:value.expected_revision,extended:value.extended,refresh:!!value.refresh}),signal:AbortSignal.timeout(15000)})
 .then(result=>{localStorage.removeItem(name);changed(frozen,result.job_id);return result;})
 .catch(error=>{const failed:DigitalSubmission={...frozen,error:requestErrorMessage(error),failure:isApiRequestError(error)&&error.status>=400&&error.status<500&&!['command_in_progress','rate_limited'].includes(error.code)?'rejected':'uncertain'};localStorage.setItem(name,JSON.stringify(failed));changed(failed);throw error;})
 .finally(()=>active.delete(name));active.set(name,attempt);return attempt;
}
export function queueDigitalAlbum(userId:string,itemId:string,title:string,expected_revision:number,extended=false,refresh=false){
 const receipt=pendingDigitalSubmissions(userId).find(value=>value.itemId===itemId&&value.failure!=='rejected')||{key:crypto.randomUUID(),expected_revision,extended,refresh,itemId,title,userId};
 return submitDigitalAlbum(receipt);
}
export function recoverDigitalSubmissions(userId:string){
 // Replay exactly the saved input once; a rejected/uncertain request remains visible for explicit retry.
 for(const receipt of pendingDigitalSubmissions(userId))if(!receipt.error||receipt.failure==='uncertain')void submitDigitalAlbum(receipt).catch(()=>{});
}

export function dismissDigitalSubmission(value:DigitalSubmission){if(value.failure!=='rejected'||active.has(storage(value)))return;localStorage.removeItem(storage(value));changed(value);}
