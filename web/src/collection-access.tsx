import React,{useEffect,useState,api,useOperationScope,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import {useDialogDismiss} from './dialog';
import type {components} from './generated/api';
import {saveGrant} from './grant-command';
import {CollectionChooser,useCollectionSearch} from './collection-chooser';

type Collection=components['schemas']['PresentedCollection'];
type Grant=components['schemas']['GrantView'];
type GrantPage=components['schemas']['Page_GrantView'];
type Feedback={kind:'success'|'error';text:string};

export default function CollectionAccess({collections,initialCollectionId,onClose,onSessionExpired}:{collections:Collection[];initialCollectionId?:string;onClose:()=>void;onSessionExpired:()=>void}){
 const dialog=useDialogDismiss(onClose);
 const manageable=collections.filter(value=>value.effective_permissions.can_manage_access);
 const [collectionId,setCollectionId]=useState(initialCollectionId&&manageable.some(value=>value.id===initialCollectionId)?initialCollectionId:manageable[0]?.id||'');
 const collectionChoices=useCollectionSearch(collectionId,{minRole:2},manageable.find(value=>value.id===collectionId),onSessionExpired);
 useEffect(()=>{if(!collectionId&&collectionChoices.items.length)setCollectionId(collectionChoices.items[0].id);},[collectionId,collectionChoices.items]);
 const [grants,setGrants]=useState<Grant[]>([]);const [loadedScope,setLoadedScope]=useState('');const [search,setSearch]=useState('');const [cursor,setCursor]=useState<string|null>(null);const [username,setUsername]=useState('');const [role,setRole]=useState<'viewer'|'curator'|'owner'>('viewer');const [feedback,setFeedback]=useState<Feedback>();const [busy,setBusy]=useState(false);const [loading,setLoading]=useState(false);
 const scope=JSON.stringify([collectionId,search]);const reads=useOperationScope(scope);const writes=useOperationScope(collectionId);
 const rowsCurrent=loadedScope===scope;
 const collection=collectionChoices.selected;
 function fail(error:unknown){if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}setFeedback({kind:'error',text:requestErrorMessage(error,'Only a collection owner can change its access. Server administration does not grant collection ownership.')});}
 const query=`/collections/${collectionId}/grants?limit=50&search=${encodeURIComponent(search)}`;
 function adopt(page:GrantPage){setGrants(page.items);setCursor(page.next_cursor??null);setLoadedScope(scope);}
 useEffect(()=>{
  if(!collectionId)return;setLoading(true);setFeedback(undefined);
  const timer=setTimeout(()=>void reads.run(attempt=>api<GrantPage>(query,{signal:attempt.signal}),{success:adopt,failure:fail,settled:()=>setLoading(false)}),150);
  return()=>{clearTimeout(timer);reads.cancel();};
 },[scope]);
 async function more(){if(!rowsCurrent||!cursor||loading)return;setLoading(true);await reads.run(attempt=>api<GrantPage>(`${query}&cursor=${encodeURIComponent(cursor)}`,{signal:attempt.signal}),{success:page=>{setGrants(previous=>[...previous,...page.items]);setCursor(page.next_cursor??null);},failure:fail,settled:()=>setLoading(false)});}
 async function save(){
  const exact=username.trim();if(!collectionId||!exact)return;setBusy(true);setFeedback(undefined);reads.cancel();
  await writes.run(async attempt=>{
   await saveGrant(attempt,collectionId,exact,role);
   return api<GrantPage>(query,{signal:attempt.signal});
  },{success:page=>{adopt(page);setUsername('');setFeedback({kind:'success',text:`Access for ${exact} saved.`});},failure:fail,settled:()=>{setBusy(false);setLoading(false);}});
 }
 async function remove(grant:Grant,origin:string){
  if(!collectionId||origin!==scope||!rowsCurrent)return;setBusy(true);setFeedback(undefined);reads.cancel();
  await writes.run(async attempt=>{
   await attempt.wait(api(`/collections/${collectionId}/grants/${encodeURIComponent(grant.username)}?expected_revision=${grant.revision}`,{method:'DELETE',signal:attempt.signal}));
   return api<GrantPage>(query,{signal:attempt.signal});
  },{success:page=>{adopt(page);setFeedback({kind:'success',text:`Access for ${grant.username} removed.`});},failure:fail,settled:()=>{setBusy(false);setLoading(false);}});
 }
 return <div className="modal-backdrop"><section ref={dialog.ref} data-unsaved={!!username} className="modal" role="dialog" aria-modal="true" aria-labelledby="access-title" aria-busy={busy}><button className="close" onClick={dialog.dismiss} aria-label="Close collection access">×</button><p className="eyebrow">Collection settings</p><h2 id="access-title">Collection access</h2><p>Collection owners manage access here, including when content is read-only or the collection has no entries.</p><CollectionChooser label="Collection" value={collectionId} onChange={setCollectionId} choices={collectionChoices} disabled={busy}/>{collection?.content_permissions.message&&<p className="notice" role="note">{collection.content_permissions.message}</p>}
 <label>Find a granted username<input disabled={busy} type="search" value={search} onChange={event=>setSearch(event.target.value)} placeholder="Search access"/></label><form onSubmit={event=>{event.preventDefault();void save();}}><div className="form-grid"><label>Exact username<input required disabled={busy} autoComplete="off" value={username} onChange={event=>setUsername(event.target.value)}/></label><label>Role<select disabled={busy} value={role} onChange={event=>setRole(event.target.value as typeof role)}><option value="viewer">Viewer</option><option value="curator">Curator</option><option value="owner">Owner</option></select></label></div><button disabled={busy||!username.trim()}>Add or update access</button></form>
 {feedback&&<p role={feedback.kind==='error'?'alert':'status'} className={`notice ${feedback.kind==='error'?'error':''}`}>{feedback.text}</p>}{loading&&<p role="status">Loading collection access…</p>}{rowsCurrent&&grants.map(grant=><p className="stored-file" key={grant.user_id}><span>{grant.username} · {grant.role}</span><button disabled={busy} onClick={()=>void remove(grant,loadedScope)}>Remove</button></p>)}{rowsCurrent&&cursor&&<button disabled={loading||busy} onClick={()=>void more()}>More people with access</button>}
 </section></div>;
}
