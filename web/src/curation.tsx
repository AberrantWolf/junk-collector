import React,{useEffect,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import InventoryEditor from './inventory-editor';
import AssetEditor from './asset-editor';
import ImportWorkspace from './import-workspace';

type Collection=components['schemas']['PresentedCollection'];
type Item=components['schemas']['PresentedItem'];
type SaveItemCommand=components['schemas']['SaveItemCommand'];
type Grant=components['schemas']['GrantView'];

type Props={collections:Collection[];selected?:Item;onClose:()=>void;onSaved:(item:Item)=>void;onSessionExpired:()=>void};
type Feedback={kind:'success'|'error';text:string};

export default function Curation({collections,selected,onClose,onSaved,onSessionExpired}:Props){
 const [item,setItem]=useState(selected);const [title,setTitle]=useState(selected?.title||'');const [subtitle,setSubtitle]=useState(selected?.subtitle||'');const [collection,setCollection]=useState(selected?.collection_id||collections[0]?.id||'');const [kind,setKind]=useState(selected?.domain.module||'org.junk.games');const [wishlist,setWishlist]=useState(selected?.wishlist||false);const [feedback,setFeedback]=useState<Feedback>();const [busy,setBusy]=useState(false);
 const [grants,setGrants]=useState<Grant[]>([]);const [grantUsername,setGrantUsername]=useState('');const [grantRole,setGrantRole]=useState<'viewer'|'curator'|'owner'>('viewer');
 const activeCollection=item?.collection_id||collection;const canManage=!!collections.find(value=>value.id===activeCollection)?.effective_permissions.can_manage_access;

 useEffect(()=>{if(canManage&&activeCollection)api<Grant[]>(`/collections/${activeCollection}/grants`).then(setGrants).catch(fail);else setGrants([]);},[activeCollection,canManage]);

 function fail(error:unknown){
  if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}
  setFeedback({kind:'error',text:requestErrorMessage(error,'You are signed in, but your collection role does not allow this change. Ask a collection owner for Curator access.')});
 }
 async function save(){
  setBusy(true);setFeedback(undefined);try{
   const command:SaveItemCommand={collection_id:item?.collection_id||collection,title,subtitle,identity_link:item?.identity_link||{kind:'none'},domain:item?{...item.domain,payload:item.domain.payload??{}}:{module:kind,schema_version:1,payload:{}},identity:item?.identity||'unidentified',translations:item?.translations||[],identifiers:item?.identifiers||[],wishlist};
   const saved=await api<Item>(`/items/${item?.id||crypto.randomUUID()}`,{method:'PUT',body:JSON.stringify({command,expected_revision:item?.revision||0})});setItem(saved);onSaved(saved);setFeedback({kind:'success',text:'Inventory saved. You can now attach artwork or original files.'});
  }catch(error){fail(error);}finally{setBusy(false);}
 }
 async function saveGrant(){if(!activeCollection||!grantUsername)return;setBusy(true);setFeedback(undefined);try{const current=grants.find(value=>value.username===grantUsername);await api(`/collections/${activeCollection}/grants/${encodeURIComponent(grantUsername)}`,{method:'PUT',body:JSON.stringify({expected_revision:current?.revision||0,role:grantRole})});setGrantUsername('');setGrants(await api<Grant[]>(`/collections/${activeCollection}/grants`));setFeedback({kind:'success',text:'Collection access updated.'});}catch(error){fail(error);}finally{setBusy(false);}}
 async function removeGrant(grant:Grant){if(!activeCollection)return;setBusy(true);try{await api(`/collections/${activeCollection}/grants/${encodeURIComponent(grant.username)}?expected_revision=${grant.revision}`,{method:'DELETE'});setGrants(await api<Grant[]>(`/collections/${activeCollection}/grants`));}catch(error){fail(error);}finally{setBusy(false);}}
 function inventorySaved(saved:Item){setItem(saved);onSaved(saved);setFeedback({kind:'success',text:'Physical inventory updated.'});}
 function assetsSaved(saved:Item){setItem(saved);onSaved(saved);setFeedback({kind:'success',text:'Supporting images updated.'});}
 return <div className="modal-backdrop"><section className="modal curation-modal" role="dialog" aria-modal="true" aria-labelledby="curation-title" aria-busy={busy}><button className="close" onClick={onClose} aria-label="Close curator">×</button><p className="eyebrow">Collection curation</p><h2 id="curation-title">{item?'Edit entry':'Add an entry'}</h2><form onSubmit={event=>{event.preventDefault();void save();}}><div className="form-grid"><label>Collection<select value={collection} disabled={!!item||busy} onChange={event=>setCollection(event.target.value)}>{collections.map(value=><option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label>Kind<select value={kind} disabled={!!item||busy} onChange={event=>setKind(event.target.value)}><option value="org.junk.games">Video game</option><option value="org.junk.music">Music CD</option></select></label></div><label>Title<input required disabled={busy} value={title} onChange={event=>setTitle(event.target.value)}/></label><label>Artist / platform / edition<input disabled={busy} value={subtitle} onChange={event=>setSubtitle(event.target.value)}/></label><div className="checks"><label><input type="checkbox" disabled={busy} checked={wishlist} onChange={event=>setWishlist(event.target.checked)}/> Wishlist</label></div><button className="primary" disabled={busy||!collection}>{item?'Save':'Add'}</button></form>{item&&<><InventoryEditor item={item} busy={busy} setBusy={setBusy} onSaved={inventorySaved} onError={fail}/><AssetEditor item={item} busy={busy} setBusy={setBusy} onSaved={assetsSaved} onError={fail}/><ImportWorkspace item={item} busy={busy} setBusy={setBusy} onError={fail}/></>}{canManage&&<><hr/><h3>Collection access</h3><div className="form-grid"><label>Exact username<input value={grantUsername} onChange={event=>setGrantUsername(event.target.value)}/></label><label>Role<select value={grantRole} onChange={event=>setGrantRole(event.target.value as typeof grantRole)}><option value="viewer">Viewer</option><option value="curator">Curator</option><option value="owner">Owner</option></select></label></div><button disabled={busy||!grantUsername} onClick={saveGrant}>Add or update access</button>{grants.map(grant=><p className="stored-file" key={grant.user_id}><span>{grant.username} · {grant.role}</span><button disabled={busy} onClick={()=>removeGrant(grant)}>Remove</button></p>)}</>}{feedback&&<p role={feedback.kind==='error'?'alert':'status'} className={`notice ${feedback.kind==='error'?'error':''}`}>{feedback.text}</p>}</section></div>;
}
