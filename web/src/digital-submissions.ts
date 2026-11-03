import {api,getApiIdentity,isApiRequestError,requestErrorMessage} from './runtime';
export type DigitalSubmission={key:string;expected_revision:number;extended:boolean;refresh?:boolean;itemId:string;title:string;userId:string;error?:string;failure?:'rejected'|'uncertain';unreadable?:boolean};
const prefix='collector.digital-match.';
const confirmed=new Map<string,string>();
const liveReceipts=new Map<string,DigitalSubmission>();
const active=new Map<string,Promise<{job_id:string}>>();
export function pendingDigitalSubmissions(userId:string):DigitalSubmission[]{
 const values:DigitalSubmission[]=[];
 let keys:string[];
 try{keys=Object.keys(localStorage).filter(key=>key.startsWith(`${prefix}${userId}.`)).slice(0,128);}catch{return [{key:'storage-unavailable',expected_revision:0,extended:false,itemId:'',title:'Saved queue requests',userId,unreadable:true,error:'Browser storage cannot be read. Inspect Activity before queueing these albums again.',failure:'uncertain'}];}
 for(const key of keys){
  const live=liveReceipts.get(key);if(live){values.push(live);continue;}
  try{const saved=JSON.parse(localStorage.getItem(key)||'null');if(!saved||typeof saved.key!=='string'||!saved.key.length||saved.key.length>128||!/^[\x21-\x7e]+$/.test(saved.key)||!Number.isSafeInteger(saved.expected_revision)||saved.expected_revision<0||typeof saved.extended!=='boolean'||saved.refresh!==undefined&&typeof saved.refresh!=='boolean'||saved.error!==undefined&&typeof saved.error!=='string')throw new Error('Invalid receipt');if(confirmed.get(key)===saved.key)continue;values.push({...saved,userId,itemId:key.slice(`${prefix}${userId}.`.length),title:saved.title||'Album',extended:!!saved.extended});}
  catch{values.push({key:`unreadable:${key}`,expected_revision:0,extended:false,itemId:key.slice(`${prefix}${userId}.`.length),title:'Saved album queue request',userId,unreadable:true,error:'Saved queue receipt is unreadable. Inspect Activity to establish its outcome before queueing this album again.',failure:'uncertain'});}
 }
 return values;
}

function storage(value:DigitalSubmission){return `${prefix}${value.userId}.${value.itemId}`;}
function changed(value:DigitalSubmission,jobId?:string){window.dispatchEvent(new CustomEvent('collector-digital-submission',{detail:{...value,jobId}}));}
export function submitDigitalAlbum(value:DigitalSubmission):Promise<{job_id:string}>{
 if(value.unreadable)return Promise.reject(new Error('The saved queue receipt cannot be replayed. Inspect Activity to establish its outcome.'));
 if(getApiIdentity()?.user_id!==value.userId)return Promise.reject(new Error('Sign in to queue this album.'));
 const name=storage(value);const existing=active.get(name);if(existing)return existing;
 const frozen={...value,error:undefined,failure:undefined};localStorage.setItem(name,JSON.stringify(frozen));confirmed.delete(name);liveReceipts.set(name,frozen);changed(frozen);
 // Admission belongs to the signed-in session, independently of album-panel lifetime.
 const attempt=api<{job_id:string}>(`/items/${value.itemId}/digital-album-matches`,{method:'POST',headers:{'Idempotency-Key':value.key},body:JSON.stringify({expected_revision:value.expected_revision,extended:value.extended,refresh:!!value.refresh}),signal:AbortSignal.timeout(15000)})
 .then(result=>{confirmed.set(name,value.key);liveReceipts.delete(name);try{localStorage.removeItem(name);}catch{/* The confirmed Job remains authoritative; replay keeps the same receipt. */}changed(frozen,result.job_id);return result;},error=>{const failed:DigitalSubmission={...frozen,error:requestErrorMessage(error),failure:isApiRequestError(error)&&error.status>=400&&error.status<500&&error.status!==408&&!['command_in_progress','rate_limited'].includes(error.code)?'rejected':'uncertain'};try{localStorage.setItem(name,JSON.stringify(failed));}catch{failed.error+= ' The queue receipt could not be saved in browser storage; keep this panel open and inspect Activity before queueing again.';}liveReceipts.set(name,failed);changed(failed);throw error;})
 .finally(()=>active.delete(name));active.set(name,attempt);return attempt;
}
export function queueDigitalAlbum(userId:string,itemId:string,title:string,expected_revision:number,extended=false,refresh=false){
 const receipt=pendingDigitalSubmissions(userId).find(value=>value.itemId===itemId&&value.failure!=='rejected')||{key:crypto.randomUUID(),expected_revision,extended,refresh,itemId,title,userId};
 return submitDigitalAlbum(receipt);
}
export function recoverDigitalSubmissions(userId:string){
 // Replay exactly the saved input once; a rejected/uncertain request remains visible for explicit retry.
 for(const receipt of pendingDigitalSubmissions(userId))if(!receipt.unreadable&&(!receipt.error||receipt.failure==='uncertain'))void submitDigitalAlbum(receipt).catch(()=>{});
}

export function dismissDigitalSubmission(value:DigitalSubmission){if(value.failure!=='rejected'&&!(value.unreadable&&value.itemId)||active.has(storage(value)))return;localStorage.removeItem(storage(value));liveReceipts.delete(storage(value));changed(value);}
