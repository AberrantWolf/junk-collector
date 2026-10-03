// Manifests can exceed sessionStorage's small quota. Original audio stays in the
// selected files; only review choices and immutable submitted inputs live here.
let database:Promise<IDBDatabase>|undefined;
function open(){
 return database??=new Promise<IDBDatabase>((resolve,reject)=>{
  const request=indexedDB.open('collector-intake-drafts',1);
  request.onupgradeneeded=()=>request.result.createObjectStore('drafts');
  request.onsuccess=()=>resolve(request.result);
  request.onerror=()=>{database=undefined;reject(request.error);};
 });
}
export async function loadDraft<T>(key:string):Promise<T|undefined>{
 const db=await open();return new Promise((resolve,reject)=>{
  const request=db.transaction('drafts','readonly').objectStore('drafts').get(key);
  request.onsuccess=()=>resolve(request.result as T|undefined);request.onerror=()=>reject(request.error);
 });
}
export async function saveDraft(key:string,value:unknown):Promise<void>{
 const db=await open();return new Promise((resolve,reject)=>{
  const tx=db.transaction('drafts','readwrite');const store=tx.objectStore('drafts');
  if(value===undefined)store.delete(key);else store.put(value,key);
  tx.oncomplete=()=>resolve();tx.onabort=()=>reject(tx.error||new Error('The import draft could not be saved.'));
 });
}
