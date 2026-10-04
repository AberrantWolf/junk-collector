import React,{useEffect,useState,api,useOperationScope,isApiRequestError,requestErrorMessage} from './runtime';
import {useDialogDismiss} from './dialog';
import type {components} from './generated/api';

type Summary=components['schemas']['CollectionListSummary'];
type ListPage={items:Summary[];next_cursor:string|null};
type Pending={stage:'create';name:string;key:string}|{stage:'append';list:Summary;key:string};
type Props={userId:string;collectionId:string;itemId:string;label:string;source?:string;track?:number;onClose:()=>void;onOpen:(id:string)=>void;onSessionExpired:()=>void};

function restorePending(storageKey:string,collectionId:string):Pending|undefined{
 try{
  const text=sessionStorage.getItem(storageKey);
  if(!text||text.length>10000)return;
  const value=JSON.parse(text) as Pending;
  if(typeof value.key!=='string')return;
  if(value.stage==='create'&&typeof value.name==='string')return value;
  if(value.stage==='append'&&value.list?.collection_id===collectionId&&typeof value.list.id==='string'&&typeof value.list.name==='string'&&Number.isSafeInteger(value.list.revision)&&value.list.revision>0)return value;
 }catch{/* A malformed or unavailable browser draft does not authorize a command. */}
}

export default function CollectionListChooser({userId,collectionId,itemId,label,source,track,onClose,onOpen,onSessionExpired}:Props){
 const storageKey=`collector.list-addition.v1.${userId}/${collectionId}/${itemId}/${source||'album'}/${track||'disc'}`;
 const [pending,setPending]=useState<Pending|undefined>(()=>restorePending(storageKey,collectionId));
 const [selected,setSelected]=useState<Summary|undefined>(()=>pending?.stage==='append'?pending.list:undefined);
 const [rows,setRows]=useState<Summary[]>([]);
 const [cursor,setCursor]=useState<string|null>(null);
 const [search,setSearch]=useState('');
 const [name,setName]=useState(()=>pending?.stage==='create'?pending.name:'');
 const [error,setError]=useState('');
 const [notice,setNotice]=useState('');
 const [busy,setBusy]=useState(false);
 const [loading,setLoading]=useState(true);
 const [reload,setReload]=useState(0);
 const dialog=useDialogDismiss(onClose);
 const reads=useOperationScope(`${collectionId}/${search}/${reload}`);
 const commands=useOperationScope(storageKey);
 // Keep the same accepted request across reload and panel reopen. A current list
 // revision is presentation state; it must never rewrite an unresolved command.
 function remember(value?:Pending){
  setPending(value);
  try{if(value)sessionStorage.setItem(storageKey,JSON.stringify(value));else sessionStorage.removeItem(storageKey);}catch{/* In-memory retries still work when browser storage is unavailable. */}
 }
 function report(e:unknown){
  if(isApiRequestError(e)&&e.status===401)onSessionExpired();
  else setError(isApiRequestError(e)&&e.status===409?'This list changed. Reload lists, review the destination, then try again.':requestErrorMessage(e,'You need Curator access to edit collection lists.'));
 }
 function applyPage(page:ListPage,append=false){
  setRows(value=>append?[...value,...page.items]:page.items);
  setCursor(page.next_cursor);
  setSelected(value=>{
   const current=page.items.find(row=>row.id===value?.id);
   return current&&(!value||current.revision>=value.revision)?current:value;
  });
 }
 useEffect(()=>{
  setLoading(true);setRows([]);setCursor(null);
  void reads.run(async attempt=>{
   const page=await attempt.wait(api<ListPage>(`/collections/${collectionId}/lists?${new URLSearchParams({search,limit:'50'})}`,{signal:attempt.signal}));
   // Reload must refresh a selected destination even when it is on a later page
   // or outside the search. Unresolved submissions retain their frozen revision.
   const current=reload&&selected&&!page.items.some(row=>row.id===selected.id)
    ?(await attempt.wait(api<{list:Summary}>(`/collections/${collectionId}/lists/${selected.id}?limit=1`,{signal:attempt.signal}))).list:undefined;
   return {page,current};
  },{
   success:({page,current})=>{applyPage(page);if(current)setSelected(value=>value?.id===current.id&&current.revision>=value.revision?current:value);},failure:report,settled:()=>setLoading(false),
  });
 },[reads]);
 async function more(){
  if(!cursor)return;
  setLoading(true);
  await reads.run(attempt=>api<ListPage>(`/collections/${collectionId}/lists?${new URLSearchParams({search,cursor,limit:'50'})}`,{signal:attempt.signal}),{
   success:page=>applyPage(page,true),failure:report,settled:()=>setLoading(false),
  });
 }
 async function append(){
  setBusy(true);setError('');setNotice('');
  await commands.run(async attempt=>{
   let submission=pending;
   if(!submission){
    if(selected)submission={stage:'append',list:selected,key:crypto.randomUUID()};
    else{
     if(!name.trim())throw new Error('Choose a list or enter a name for a new list.');
     submission={stage:'create',name:name.trim(),key:crypto.randomUUID()};
    }
    remember(submission);
   }
   if(submission.stage==='create'){
    const list=await attempt.wait(api<Summary>(`/collections/${collectionId}/lists`,{method:'POST',signal:attempt.signal,headers:{'Idempotency-Key':submission.key},body:JSON.stringify({name:submission.name})}));
    setSelected(list);
    submission={stage:'append',list,key:crypto.randomUUID()};
    remember(submission);
   }
   const {list,key}=submission;
   return api<Summary>(`/collections/${collectionId}/lists/${list.id}`,{method:'PUT',signal:attempt.signal,headers:{'Idempotency-Key':key},body:JSON.stringify({expected_revision:list.revision,change:{action:'append',item_id:itemId,source_representation_id:source||null,track_position:track||null}})});
  },{
   success:updated=>{
    remember();setName('');setSelected(updated);
    setRows(value=>value.map(row=>row.id===updated.id?updated:row));
    setNotice(`${label} added to ${updated.name}.`);
   },
   failure:e=>{
    // These responses establish rejection, so a revised command is safe. Other
    // failures may have followed a commit; retry the original receipt first.
    if(isApiRequestError(e)&&[400,404,409,422].includes(e.status))remember();
    report(e);
   },
   settled:()=>setBusy(false),
  });
 }
 const options=selected&&!rows.some(row=>row.id===selected.id)?[selected,...rows]:rows;
 return <div className="modal-backdrop"><section ref={dialog.ref} className="modal" role="dialog" aria-modal="true" aria-labelledby="list-chooser-title" aria-busy={busy} data-unsaved={!pending&&!selected&&!!name.trim()}>
  <button className="close" aria-label="Close" onClick={dialog.dismiss}>×</button>
  <h2 id="list-chooser-title">Add to list</h2>
  <p>{label} will be appended at the end. Repeated additions are allowed.</p>
  <label>Find a list<input type="search" value={search} onChange={e=>setSearch(e.target.value)} maxLength={256}/></label>
  <label>Destination list<select value={selected?.id||''} disabled={busy||!!pending} onChange={e=>{setSelected(rows.find(row=>row.id===e.target.value));setNotice('');}}>
   <option value="">Create a new list</option>
   {options.filter(row=>row.can_edit||row.id===selected?.id).map(row=><option key={row.id} value={row.id}>{row.name} ({row.entry_count})</option>)}
  </select></label>
  {cursor&&<button disabled={loading||busy} onClick={()=>void more()}>More lists</button>}
  {!selected&&<label>New list name<input value={name} maxLength={256} disabled={busy||!!pending} onChange={e=>setName(e.target.value)}/></label>}
  {loading&&<p role="status">Loading lists…</p>}
  {pending&&!busy&&<p className="notice">The previous addition has not been confirmed. Retry addition checks that same request before adding anything else.</p>}
  {error&&<p role="alert" className="error">{error}<button disabled={busy} onClick={()=>setReload(value=>value+1)}>Reload lists</button></p>}
  {notice&&<p role="status">{notice}</p>}
  <div className="profile-actions">
   <button className="primary" disabled={busy||loading||!pending&&!selected&&!name.trim()} onClick={()=>void append()}>{busy?'Adding…':pending?'Retry addition':'Append to list'}</button>
   {notice&&selected&&<button onClick={()=>onOpen(selected.id)}>Open list</button>}
  </div>
 </section></div>;
}
