import React,{useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {Collection,Item} from './types';

type Props={collections:Collection[];selected?:Item;onClose:()=>void;onSaved:(item:Item)=>void;onSessionExpired:()=>void};
type Feedback={kind:'success'|'error';text:string};

export default function Curation({collections,selected,onClose,onSaved,onSessionExpired}:Props){
 const [item,setItem]=useState(selected);const [title,setTitle]=useState(selected?.title||'');const [subtitle,setSubtitle]=useState(selected?.subtitle||'');const [collection,setCollection]=useState(selected?.collection_id||collections[0]?.id||'');const [kind,setKind]=useState(selected?.domain.module||'org.junk.games');const [wishlist,setWishlist]=useState(selected?.wishlist||false);const [physical,setPhysical]=useState(false);const [feedback,setFeedback]=useState<Feedback>();const [busy,setBusy]=useState(false);
 const [uploadFiles,setUploadFiles]=useState<File[]>([]);const [cover,setCover]=useState<File>();

 function fail(error:unknown){
  if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}
  setFeedback({kind:'error',text:requestErrorMessage(error,'You are signed in, but your collection role does not allow this change. Ask a collection owner for Curator access.')});
 }
 async function save(){
  setBusy(true);setFeedback(undefined);try{
   const next:Item=item?{...item,title,subtitle,wishlist}:{id:crypto.randomUUID(),collection_id:collection,revision:0,title,subtitle,identity_link:{kind:'none'},domain:{module:kind,schema_version:1,payload:{}},identity:'unidentified',translations:[],identifiers:[],specimens:[],wishlist,captures:[],assets:[],metadata:{candidates:[],selected:null,overrides:{}},deleted:false,updated_at:new Date().toISOString()};
   if(physical)next.specimens=[...next.specimens,{id:crypto.randomUUID(),label:'Owned copy',condition:'unknown',ownership:{state:'owned'},possession:'present',components:[],acquisition:null,private_notes:null,loans:[]}];
   const saved=await api<Item>(`/items/${next.id}`,{method:'PUT',body:JSON.stringify({item:next,expected_revision:item?.revision||0})});setItem(saved);setPhysical(false);onSaved(saved);setFeedback({kind:'success',text:'Inventory saved. You can now attach artwork or original files.'});
  }catch(error){fail(error);}finally{setBusy(false);}
 }
 async function attachCover(){
  if(!item||!cover)return;setBusy(true);setFeedback(undefined);try{const saved=await api<Item>(`/items/${item.id}/cover?expected_revision=${item.revision}&alt_text=${encodeURIComponent(title+' cover')}`,{method:'POST',body:cover});setItem(saved);onSaved(saved);setCover(undefined);setFeedback({kind:'success',text:'Original artwork and thumbnail saved.'});}catch(error){fail(error);}finally{setBusy(false);}
 }
 async function upload(){
  if(!item||!uploadFiles.length)return;setBusy(true);setFeedback(undefined);try{
   const created=await api<{id:string;offsets:number[]}>(`/uploads`,{method:'POST',body:JSON.stringify({item_id:item.id,expected_revision:item.revision,files:uploadFiles.map(file=>({name:file.webkitRelativePath||file.name,bytes:file.size,sha256:null}))})});
   for(let index=0;index<uploadFiles.length;index++){const file=uploadFiles[index];for(let offset=created.offsets[index];offset<file.size;offset+=4*1024*1024){setFeedback({kind:'success',text:`Uploading ${file.name}: ${Math.round(offset/file.size*100)}%`});await api(`/uploads/${created.id}/files/${index}?offset=${offset}`,{method:'PUT',body:file.slice(offset,offset+4*1024*1024)});}}
   await api(`/uploads/${created.id}/commit`,{method:'POST'});setFeedback({kind:'success',text:'Upload accepted. The server is verifying and publishing the archive copy.'});
  }catch(error){fail(error);}finally{setBusy(false);}
 }
 return <div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-labelledby="curation-title" aria-busy={busy}><button className="close" onClick={onClose} aria-label="Close curator">×</button><p className="eyebrow">Collection curation</p><h2 id="curation-title">{item?'Edit entry':'Add an entry'}</h2><form onSubmit={event=>{event.preventDefault();void save();}}><div className="form-grid"><label>Collection<select value={collection} disabled={!!item||busy} onChange={event=>setCollection(event.target.value)}>{collections.map(value=><option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label>Kind<select value={kind} disabled={!!item||busy} onChange={event=>setKind(event.target.value)}><option value="org.junk.games">Video game</option><option value="org.junk.music">Music CD</option></select></label></div><label>Title<input required disabled={busy} value={title} onChange={event=>setTitle(event.target.value)}/></label><label>Artist / platform / edition<input disabled={busy} value={subtitle} onChange={event=>setSubtitle(event.target.value)}/></label><div className="checks"><label><input type="checkbox" disabled={busy} checked={wishlist} onChange={event=>setWishlist(event.target.checked)}/> Wishlist</label><label><input type="checkbox" disabled={busy} checked={physical} onChange={event=>setPhysical(event.target.checked)}/> Add an owned physical copy</label></div><button className="primary" disabled={busy||!collection}>{item?'Save':'Add'}</button></form>{item&&<><hr/><h3>Cover artwork</h3><label>PNG, JPEG, or WebP<input type="file" disabled={busy} accept="image/png,image/jpeg,image/webp" onChange={event=>setCover(event.target.files?.[0])}/></label><button disabled={busy||!cover} onClick={attachCover}>Attach cover</button><hr/><h3>Original dump package</h3><p className="muted">Choose all files in the package, including CUE sheets and dump sidecars. Your local originals remain in place.</p><input aria-label="Original package files" type="file" disabled={busy} multiple onChange={event=>setUploadFiles(Array.from(event.target.files||[]))}/><button disabled={busy||!uploadFiles.length} onClick={upload}>Upload and preserve</button></>}{feedback&&<p role={feedback.kind==='error'?'alert':'status'} className={`notice ${feedback.kind==='error'?'error':''}`}>{feedback.text}</p>}</section></div>;
}
