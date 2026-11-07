import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import {useDialogDismiss} from './dialog';

type S=components['schemas'];
export type AlbumTarget={id:string;title:string};
const fields=[
 {key:'digital_source',label:'Digital source',maxLength:256},
 {key:'artist',label:'Album artist',maxLength:4096},
 {key:'title',label:'Album title',maxLength:4096},
] as const;
type Field=typeof fields[number]['key'];

export default function ImportAlbumEditor({albums,onClose,onChanged,onSessionExpired}:{
 albums:AlbumTarget[];onClose:()=>void;onChanged:()=>void;onSessionExpired:()=>void;
}){
 const [draft,setDraft]=useState<Partial<Record<Field,string>>>({});
 const [remaining,setRemaining]=useState(albums);
 const [results,setResults]=useState<{album:AlbumTarget;message:string}[]>([]);
 const [saved,setSaved]=useState(0);
 const [progress,setProgress]=useState<number>();
 const work=useRef<AbortController|null>(null);
 const busy=progress!==undefined;
 const dialog=useDialogDismiss(onClose);
 useEffect(()=>()=>work.current?.abort(),[]);
 const chosen=fields.filter(({key})=>key in draft);

 async function save(){
  if(work.current||!chosen.length||chosen.some(({key})=>!draft[key]?.trim()))return;
  const controller=new AbortController();work.current=controller;
  const {signal}=controller;
  const change:S['MetadataAuthorityChange']={change:'set_album_overrides'};
  for(const {key} of chosen)change[key]=draft[key]!.trim();
  const failed:typeof results=[];
  let completed=0;
  setProgress(0);setResults([]);
  try{
   for(const album of remaining){
    try{
     const path=`/items/${album.id}/metadata`;
     const review=await api<S['MetadataReviewView']>(path,{signal});
     if(!review.permissions.can_update)throw new Error('You need permission to edit this album.');
     if(chosen.some(({key})=>key==='digital_source')&&!review.digital_source_eligible)throw new Error('Digital source requires a digital-only album.');
     const manual=review.authority.field_overrides;
     const current={
      title:manual.find(field=>field.field==='title')?.value,
      artist:manual.find(field=>field.field==='subtitle')?.value,
      digital_source:review.display.domain_fields?.digital_source?.value,
     };
     if(chosen.some(({key})=>change[key]!==current[key])){
      const command:S['UpdateMetadataAuthorityCommand']={expected_revision:review.revision,change};
      await api(path,{method:'PUT',signal,body:JSON.stringify(command)});
     }
     completed++;
    }catch(error){
     if(signal.aborted)return;
     if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}
     failed.push({album,message:requestErrorMessage(error)});
    }
    setProgress(value=>(value??0)+1);
   }
   setResults(failed);
   setRemaining(failed.map(result=>result.album));
   setSaved(value=>value+completed);
   if(completed)onChanged();
  }finally{
   work.current=null;
   if(!signal.aborted)setProgress(undefined);
  }
 }

 return <div className="modal-backdrop"><section className="modal curation-modal" role="dialog" aria-modal="true" aria-label="Edit selected albums" aria-busy={busy} ref={dialog.ref}>
  <button type="button" className="close" aria-label="Close album editor" onClick={dialog.dismiss}>×</button>
  <h2>Edit selected albums</h2>
  {remaining.length>0&&<form onSubmit={event=>{event.preventDefault();void save();}}>
   <p>Choose which fields to overwrite for {remaining.length} {remaining.length===1?'album':'albums'}. Digital source applies only to digital-only albums.</p>
   <fieldset className="intake-album-fields" disabled={busy}>
    <legend>Album metadata</legend>
    {fields.map(({key,label,maxLength})=><div key={key}>
     <label className="intake-select"><input type="checkbox" checked={key in draft} onChange={event=>{
      const checked=event.target.checked;
      setDraft(current=>{const next={...current};if(checked)next[key]='';else delete next[key];return next;});
     }}/> Overwrite {label.toLowerCase()}</label>
     {key in draft&&<label>{label}<input required maxLength={maxLength} value={draft[key]} onChange={event=>setDraft(current=>({...current,[key]:event.target.value}))}/></label>}
    </div>)}
   </fieldset>
   <button type="submit" className="primary" disabled={busy||!chosen.length||chosen.some(({key})=>!draft[key]?.trim())}>{results.length?'Retry remaining albums':'Save album changes'}</button>
  </form>}
  {busy&&<p role="status">Processed {progress} of {remaining.length} albums…</p>}
  {!busy&&saved>0&&<p role="status">{saved} {saved===1?'album':'albums'} saved.</p>}
  {results.length>0&&<div className="notice error" role="alert"><p>Could not confirm changes for {results.length} {results.length===1?'album':'albums'}.</p>
   <ul>{results.map(({album,message})=><li key={album.id}>{album.title} · {message}</li>)}</ul>
  </div>}
  {remaining.length===0&&<button type="button" onClick={onClose}>Done</button>}
 </section></div>;
}
