import {useState} from '@collector/runtime';
import type {components} from './generated/api';
import {api,isApiRequestError,requestErrorMessage} from './runtime';
import {useDialogDismiss} from './dialog';
import {bytes} from './work-progress';

type Item=components['schemas']['PresentedItem'];
type Purge=components['schemas']['ItemPurgeResult'];
type Choice='hide'|'files'|'everything';

const combinedNote='This album was combined from other entries. After this, that combine can no longer be undone.';

function originalBytes(item:Item,captures?:string[]){
 return item.captures.filter(capture=>!captures||captures.includes(capture.id)).flatMap(capture=>capture.representations).flatMap(representation=>representation.files).reduce((total,file)=>total+file.bytes,0);
}

// Work on the entry stops or finishes before files can go; retry while it does.
async function withRetry<T>(request:()=>Promise<T>,refresh:()=>Promise<void>){
 for(let attempt=0;;attempt++){
  try{return await request();}
  catch(failure){
   if(!(isApiRequestError(failure)&&failure.status===409)||attempt>=15)throw failure;
   await new Promise(resolve=>setTimeout(resolve,1000));
   await refresh();
  }
 }
}

// Removing an entry hides it and can be undone. The other choices delete for
// good: only its files (the entry and its copy records stay, for example after
// giving a copy away), or everything. An already removed entry can only be
// deleted entirely.
export function RemoveEntryDialog({item,onClose,onDone,onSessionExpired}:{item:Item;onClose:()=>void;onDone:(result:{removed?:Item;kept?:boolean;freed?:number})=>void;onSessionExpired:()=>void}){
 const already=item.deleted;
 const [choice,setChoice]=useState<Choice>(already?'everything':'hide');
 const [busy,setBusy]=useState(false);
 const [error,setError]=useState('');
 // Set when this dialog removed the entry but the deletion after it failed:
 // the dialog stays open with the error, a retry skips the removal, and
 // closing reports the removal.
 const [removedHere,setRemovedHere]=useState<Item>();
 const dialog=useDialogDismiss(()=>{if(busy)return;if(removedHere)onDone({removed:removedHere});else onClose();});
 const size=originalBytes(item);
 async function confirm(){
  setBusy(true);setError('');
  let removed:Item|undefined;
  try{
   if(choice==='files'){
    let current=item;
    const result=await withRetry(()=>api<Purge>(`/items/${item.id}/files/purge`,{method:'POST',body:JSON.stringify({expected_revision:current.revision,captures:current.captures.map(capture=>capture.id),remove_copies:false})}),async()=>{current=await api<Item>(`/items/${item.id}`);});
    onDone({kept:true,freed:result.freed_bytes});return;
   }
   removed=already?item:removedHere??await api<Item>(`/items/${item.id}/remove`,{method:'POST',body:JSON.stringify({expected_revision:item.revision})});
   if(choice==='hide'){onDone({removed});return;}
   if(!already)setRemovedHere(removed);
   let current=removed;
   const result=await withRetry(()=>api<Purge>(`/items/${item.id}/purge`,{method:'POST',body:JSON.stringify({expected_revision:current.revision})}),async()=>{current=await api<Item>(`/items/${item.id}`);});
   onDone({removed,freed:result.freed_bytes});
  }catch(failure){
   if(isApiRequestError(failure)&&failure.code==='unauthenticated'){onSessionExpired();return;}
   const message=requestErrorMessage(failure);
   setError(removed&&!already?`${message} The entry is removed and can still be restored.`:message);
  }finally{setBusy(false);}
 }
 const option=(value:Choice,title:string,detail:string,disabled=false)=><label className={`remove-entry-option ${choice===value?'checked':''} ${value==='hide'?'':'permanent'}`}>
  <input type="radio" name="remove-entry-choice" value={value} checked={choice===value} disabled={busy||disabled} onChange={()=>setChoice(value)}/>
  <span><strong>{title}</strong><small>{detail}</small></span>
 </label>;
 const label=choice==='hide'?'Remove entry':choice==='files'?'Delete files permanently':already?'Delete permanently':'Remove and delete permanently';
 return <div className="modal-backdrop"><section ref={dialog.ref} className="modal remove-entry" role="dialog" aria-modal="true" aria-labelledby="remove-entry-title" aria-busy={busy}>
  <button className="close" aria-label="Close" disabled={busy} onClick={dialog.dismiss}>×</button>
  <h2 id="remove-entry-title">{already?`Delete “${item.title}” permanently?`:`Remove “${item.title}”?`}</h2>
  {already?<p>The entry, its files{size?` (${bytes(size)})`:''}, saved copies, covers, copy records, metadata and import history are deleted for good. Works, editions and lists it appeared in are kept.</p>:
  <div className="remove-entry-choices" role="radiogroup" aria-label="What to remove">
   {option('hide','Hide it','The entry is hidden and its running work stops. You can restore it later.')}
   {option('files','Delete its files, keep the entry',`Its original files${size?` (${bytes(size)})`:''}, saved FLAC and Opus copies and check results are deleted for good. The entry and its physical copy records stay, for example after giving a copy away.`,item.captures.length===0||!!removedHere)}
   {option('everything','Delete everything',`The entry, its files${size?` (${bytes(size)})`:''}, saved copies, covers, copy records, metadata and import history are deleted for good. Use this for an entry imported by mistake. Works, editions and lists it appeared in are kept.`)}
  </div>}
  {choice!=='hide'&&<p className="notice">This can’t be undone.{item.merge_group?` ${combinedNote}`:''}</p>}
  {error&&<p role="alert" className="notice error">{error}</p>}
  <div className="button-row">
   <button disabled={busy} onClick={dialog.dismiss}>Cancel</button>
   <button className={choice==='hide'?'primary':'danger'} disabled={busy} onClick={()=>void confirm()}>{busy?(choice==='hide'?'Removing…':'Deleting…'):label}</button>
  </div>
 </section></div>;
}

// Deletes one original (a disc or a purchase) from an entry that stays. The
// physical copy it came from keeps its record unless the box is ticked, and
// only when no other original of that copy remains.
export function DeleteFilesDialog({item,captureId,title,onClose,onDone,onSessionExpired}:{item:Item;captureId:string;title:string;onClose:()=>void;onDone:(freed:number)=>void;onSessionExpired:()=>void}){
 const [removeCopy,setRemoveCopy]=useState(false);
 const [busy,setBusy]=useState(false);
 const [error,setError]=useState('');
 const dialog=useDialogDismiss(()=>{if(!busy)onClose();});
 const capture=item.captures.find(value=>value.id===captureId);
 const specimen=capture?.association.specimen_id;
 const copy=specimen?item.specimens.find(value=>value.id===specimen):undefined;
 const shared=!!specimen&&item.captures.some(value=>value.id!==captureId&&value.association.specimen_id===specimen);
 const size=originalBytes(item,[captureId]);
 async function confirm(){
  setBusy(true);setError('');
  try{
   let current=item;
   const result=await withRetry(()=>api<Purge>(`/items/${item.id}/files/purge`,{method:'POST',body:JSON.stringify({expected_revision:current.revision,captures:[captureId],remove_copies:removeCopy})}),async()=>{current=await api<Item>(`/items/${item.id}`);});
   onDone(result.freed_bytes);
  }catch(failure){
   if(isApiRequestError(failure)&&failure.code==='unauthenticated'){onSessionExpired();return;}
   setError(requestErrorMessage(failure));
  }finally{setBusy(false);}
 }
 return <div className="modal-backdrop"><section ref={dialog.ref} className="modal remove-entry" role="dialog" aria-modal="true" aria-labelledby="delete-files-title" aria-busy={busy}>
  <button className="close" aria-label="Close" disabled={busy} onClick={dialog.dismiss}>×</button>
  <h2 id="delete-files-title">Delete {title}?</h2>
  <p>Its original files{size?` (${bytes(size)})`:''}, saved FLAC and Opus copies and check results are deleted for good. The rest of “{item.title}” stays.</p>
  {copy&&!shared&&<label className={`remove-entry-option permanent ${removeCopy?'checked':''}`}>
   <input type="checkbox" checked={removeCopy} disabled={busy} onChange={event=>setRemoveCopy(event.target.checked)}/>
   <span><strong>Also delete the copy record “{copy.label}”</strong><small>Leave this clear if you gave the copy away or still own it, so your inventory keeps it. Tick it if this was the same disc imported twice.</small></span>
  </label>}
  {copy&&shared&&<p className="muted">The copy record “{copy.label}” stays, because its other discs are still here.</p>}
  <p className="notice">This can’t be undone.{item.merge_group?` ${combinedNote}`:''}</p>
  {error&&<p role="alert" className="notice error">{error}</p>}
  <div className="button-row">
   <button disabled={busy} onClick={dialog.dismiss}>Cancel</button>
   <button className="danger" disabled={busy} onClick={()=>void confirm()}>{busy?'Deleting…':'Delete files permanently'}</button>
  </div>
 </section></div>;
}
