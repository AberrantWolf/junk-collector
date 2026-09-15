import React,{useEffect,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import InventoryEditor from './inventory-editor';
import AssetEditor from './asset-editor';
import ImportWorkspace from './import-workspace';
import ProcessingWorkspace from './processing-workspace';
import JobLedger from './job-ledger';
import MusicTracks from './music-tracks';
import {useDialogDismiss} from './dialog';

type Collection=components['schemas']['PresentedCollection'];
type CollectionType=components['schemas']['ModuleDescriptor'];
type DomainField=components['schemas']['DomainFieldDescriptor'];
type Item=components['schemas']['PresentedItem'];
type SaveItemEditCommand=components['schemas']['SaveItemEditCommand'];
type Grant=components['schemas']['GrantView'];

type Props={initialCollectionId?:string;collections:Collection[];collectionTypes:CollectionType[];selected?:Item;onClose:()=>void;onSaved:(item:Item)=>void;onSessionExpired:()=>void};
type Feedback={kind:'success'|'error';text:string};

function scalarValues(item?:Item):Record<string,string>{const payload=item?.domain.payload;if(!payload||typeof payload!=='object'||Array.isArray(payload))return {};return Object.fromEntries(Object.entries(payload).filter(([,value])=>typeof value==='string'||typeof value==='number').map(([key,value])=>[key,String(value)]));}

export default function Curation({initialCollectionId,collections,collectionTypes,selected,onClose,onSaved,onSessionExpired}:Props){
 const dialog=useDialogDismiss(onClose);
 const [item,setItem]=useState(selected);const [title,setTitle]=useState(selected?.title||'');const [subtitle,setSubtitle]=useState(selected?.subtitle||'');const [collection,setCollection]=useState(selected?.collection_id||initialCollectionId||collections[0]?.id||'');const [fieldValues,setFieldValues]=useState<Record<string,string>>(()=>scalarValues(selected));const [domainPatch,setDomainPatch]=useState<Record<string,unknown>>({});const [wishlist,setWishlist]=useState(selected?.wishlist||false);const [feedback,setFeedback]=useState<Feedback>();const [busy,setBusy]=useState(false);
 const [grants,setGrants]=useState<Grant[]>([]);const [grantUsername,setGrantUsername]=useState('');const [grantRole,setGrantRole]=useState<'viewer'|'curator'|'owner'>('viewer');
 const activeCollection=item?.collection_id||collection;const canManage=!!collections.find(value=>value.id===activeCollection)?.effective_permissions.can_manage_access;
 const selectedCollection=collections.find(value=>value.id===activeCollection);const owner=selectedCollection?.binding.state==='assigned'?selectedCollection.binding.module:'';const descriptor=collectionTypes.find(value=>value.id===owner);const canEdit=!!selectedCollection?.content_permissions.can_edit&&(!item||item.content_permissions.can_edit);

 useEffect(()=>{if(canManage&&activeCollection)api<Grant[]>(`/collections/${activeCollection}/grants`).then(setGrants).catch(fail);else setGrants([]);},[activeCollection,canManage]);

 function fail(error:unknown){
  if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}
  setFeedback({kind:'error',text:requestErrorMessage(error,'You are signed in, but your collection role does not allow this change. Ask a collection owner for Curator access.')});
 }
 async function save(){
  setBusy(true);setFeedback(undefined);try{
   if(!canEdit)throw new Error('Content editing is unavailable for this collection or item.');
   for(const field of descriptor?.fields||[]){if(field.kind.type==='integer'&&domainPatch[field.key]!==undefined&&domainPatch[field.key]!==null&&!Number.isSafeInteger(domainPatch[field.key]))throw new Error(`${field.label} must be a safely representable whole number.`);}
   if(!descriptor)throw new Error('The selected collection type is unavailable. Preserved content remains read-only.');
   const edit:SaveItemEditCommand={collection_id:item?.collection_id||collection,title,subtitle,identity_link:item?.identity_link||{kind:'none'},domain_edit:{module:descriptor.id,fields:domainPatch as never},identity:item?.identity||'unidentified',translations:item?.translations||[],identifiers:item?.identifiers||[],wishlist};
   const saved=await api<Item>(`/items/${item?.id||crypto.randomUUID()}`,{method:'PUT',body:JSON.stringify({edit,expected_revision:item?.revision||0})});setItem(saved);setFieldValues(scalarValues(saved));setDomainPatch({});onSaved(saved);setFeedback({kind:'success',text:'Inventory saved. You can now attach artwork or original files.'});
  }catch(error){fail(error);}finally{setBusy(false);}
 }
 async function saveGrant(){if(!activeCollection||!grantUsername)return;setBusy(true);setFeedback(undefined);try{const current=grants.find(value=>value.username===grantUsername);await api(`/collections/${activeCollection}/grants/${encodeURIComponent(grantUsername)}`,{method:'PUT',body:JSON.stringify({expected_revision:current?.revision||0,role:grantRole})});setGrantUsername('');setGrants(await api<Grant[]>(`/collections/${activeCollection}/grants`));setFeedback({kind:'success',text:'Collection access updated.'});}catch(error){fail(error);}finally{setBusy(false);}}
 async function removeGrant(grant:Grant){if(!activeCollection)return;setBusy(true);try{await api(`/collections/${activeCollection}/grants/${encodeURIComponent(grant.username)}?expected_revision=${grant.revision}`,{method:'DELETE'});setGrants(await api<Grant[]>(`/collections/${activeCollection}/grants`));}catch(error){fail(error);}finally{setBusy(false);}}
 function inventorySaved(saved:Item){setItem(saved);onSaved(saved);setFeedback({kind:'success',text:'Physical inventory updated.'});}
 function capturePublished(saved:Item){setItem(saved);onSaved(saved);setFeedback({kind:'success',text:saved.domain.module==='org.junk.music'?'Original files preserved. If attached to this album, prepare FLAC tracks in Music preparation above. Album lookup is not automatic.':'Capture published. Package operations are now available below.'});}
 function assetsSaved(saved:Item){setItem(saved);onSaved(saved);setFeedback({kind:'success',text:'Supporting images updated.'});}
 function changeCollection(id:string){setCollection(id);setFieldValues({});setDomainPatch({});setFeedback({kind:'success',text:'Collection changed. Unsaved type-specific fields were cleared.'});}
 function changeField(field:DomainField,value:string){setFieldValues(current=>({...current,[field.key]:value}));setDomainPatch(current=>({...current,[field.key]:value===''?null:field.kind.type==='integer'?Number(value):value}));}
 return <div className="modal-backdrop"><section ref={dialog.ref} data-unsaved={title!==(item?.title||'')||subtitle!==(item?.subtitle||'')||wishlist!==(item?.wishlist||false)||Object.keys(domainPatch).some(key=>fieldValues[key]!== (scalarValues(item)[key]||''))||!!grantUsername} className="modal curation-modal" role="dialog" aria-modal="true" aria-labelledby="curation-title" aria-busy={busy}><button className="close" onClick={dialog.dismiss} aria-label="Close curator">×</button><p className="eyebrow">Collection curation</p><h2 id="curation-title">{item?'Edit entry':'Add an entry'}</h2>{(item?.content_permissions.message||selectedCollection?.content_permissions.message)&&<p className="notice" role="note">{item?.content_permissions.message||selectedCollection?.content_permissions.message}</p>}{feedback&&<p role={feedback.kind==='error'?'alert':'status'} className={`notice ${feedback.kind==='error'?'error':''}`}>{feedback.text}</p>}{item?.domain.module==='org.junk.music'&&<><p className="notice">Automatic album names, track titles and cover lookup are not implemented yet. Enter album details here and attach cover art below.</p><MusicTracks key={item.id} item={item} canPrepare onSessionExpired={onSessionExpired}/></>}<form onSubmit={event=>{event.preventDefault();void save();}}><label>Collection<select value={collection} disabled={!!item||busy} onChange={event=>changeCollection(event.target.value)}>{collections.map(value=><option key={value.id} value={value.id}>{value.name}</option>)}</select></label>{descriptor&&<p className="type-label"><strong>{descriptor.collection_type_label}</strong> fields are supplied by {descriptor.display_name}.</p>}<label>Title<input required disabled={busy||!descriptor||!canEdit} value={title} onChange={event=>setTitle(event.target.value)}/></label><label>Edition / variant details<input disabled={busy||!descriptor||!canEdit} value={subtitle} onChange={event=>setSubtitle(event.target.value)}/></label>{descriptor?.fields.map(field=><DomainFieldInput key={field.key} field={field} value={fieldValues[field.key]||''} busy={busy||!canEdit} onChange={value=>changeField(field,value)}/>)}<div className="checks"><label><input type="checkbox" disabled={busy||!descriptor||!canEdit} checked={wishlist} onChange={event=>setWishlist(event.target.checked)}/> Wishlist</label></div><button className="primary" disabled={busy||!collection||!descriptor||!canEdit}>{item?'Save':'Add'}</button></form>{item&&item.content_permissions.can_edit&&<><InventoryEditor item={item} busy={busy} setBusy={setBusy} onSaved={inventorySaved} onError={fail}/><AssetEditor item={item} busy={busy} setBusy={setBusy} onSaved={assetsSaved} onError={fail}/><ImportWorkspace item={item} busy={busy} setBusy={setBusy} onPublished={capturePublished} onError={fail}/><ProcessingWorkspace item={item} busy={busy} setBusy={setBusy} onError={fail}/><JobLedger/></>}{canManage&&<><hr/><h3>Collection access</h3><div className="form-grid"><label>Exact username<input value={grantUsername} onChange={event=>setGrantUsername(event.target.value)}/></label><label>Role<select value={grantRole} onChange={event=>setGrantRole(event.target.value as typeof grantRole)}><option value="viewer">Viewer</option><option value="curator">Curator</option><option value="owner">Owner</option></select></label></div><button disabled={busy||!grantUsername} onClick={saveGrant}>Add or update access</button>{grants.map(grant=><p className="stored-file" key={grant.user_id}><span>{grant.username} · {grant.role}</span><button disabled={busy} onClick={()=>removeGrant(grant)}>Remove</button></p>)}</>}</section></div>;
}

function DomainFieldInput({field,value,busy,onChange}:{field:DomainField;value:string;busy:boolean;onChange:(value:string)=>void}){
 if(field.kind.type==='structured')return <div className="read-only-field"><strong>{field.label}</strong><span>Structured module data is preserved and read-only here.</span></div>;
 if(!field.editable)return <div className="read-only-field"><strong>{field.label}</strong><span>This module field is preserved and read-only.</span></div>;
 return <label>{field.label}<input disabled={busy} required={field.required} type={field.kind.type==='integer'?'number':'text'} min={field.kind.type==='integer'?field.kind.minimum:undefined} max={field.kind.type==='integer'?field.kind.maximum:undefined} maxLength={field.kind.type==='text'?field.kind.maximum_bytes:undefined} value={value} onChange={event=>onChange(event.target.value)}/></label>;
}
