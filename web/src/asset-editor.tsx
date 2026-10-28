import React,{useEffect,useRef,useState,useId,api} from '@collector/runtime';
import AssetUpload from './asset-upload';
import {DocumentActions} from './document-attachments';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type Asset=components['schemas']['PresentedAsset'];
type Visibility=components['schemas']['Visibility'];
type Props={mode?:'images'|'documents';onSessionExpired?:()=>void;item:Item;busy:boolean;setBusy:(value:boolean)=>void;onSaved:(item:Item)=>void;onError:(error:unknown)=>void};
function attachmentLabel(item:Item,asset:Asset):string{
 const subject=asset.subject;
 if(subject.kind==='item')return `Item · ${item.title}`;
 if(subject.kind==='work')return 'Linked work';
 if(subject.kind==='edition')return 'Linked edition';
 if(subject.kind==='capture')return `Capture ${item.captures.findIndex(value=>value.id===subject.capture_id)+1}`;
 for(const [copyIndex,specimen] of item.specimens.entries()){
  if(subject.kind==='specimen'&&subject.specimen_id===specimen.id)return `Copy ${copyIndex+1} · ${specimen.label}`;
  if(subject.kind==='component'){
   const component=specimen.components.find(value=>value.id===subject.component_id);
   if(component)return `Copy ${copyIndex+1} · Part ${component.position} · ${component.label}`;
  }
 }
 return subject.kind.replace('_',' ');
}

function AssetRow({item,asset,busy,setBusy,onChanged,onError,restoreFocus,onSessionExpired}:{onSessionExpired:()=>void;item:Item;asset:Asset;busy:boolean;setBusy:(value:boolean)=>void;onChanged:()=>Promise<void>;onError:(error:unknown)=>void;restoreFocus:()=>void}){
 const [editing,setEditing]=useState(false);const [visibility,setVisibility]=useState<Visibility>(asset.visibility||'shared');const [altText,setAltText]=useState(asset.alt_text);
 useEffect(()=>{setVisibility(asset.visibility||'shared');setAltText(asset.alt_text);},[asset.revision]);
 async function save(){setBusy(true);try{await api(`/collections/${item.collection_id}/assets/${asset.id}/metadata`,{method:'PUT',body:JSON.stringify({expected_revision:asset.revision,command:{visibility,alt_text:altText}})});await onChanged();setEditing(false);}catch(error){onError(error);}finally{setBusy(false);}}
 async function archive(){if(!confirm(`Archive this ${asset.kind}? The original stays saved in revision history.`))return;let archived=false;setBusy(true);try{await api(`/collections/${item.collection_id}/assets/${asset.id}/archive?expected_revision=${asset.revision}`,{method:'POST'});await onChanged();archived=true;}catch(error){onError(error);}finally{setBusy(false);if(archived)requestAnimationFrame(restoreFocus);}}
 return <article data-unsaved={editing&&(visibility!==(asset.visibility||'shared')||altText!==asset.alt_text)} className="asset-record" aria-label={`${asset.kind} attached to ${attachmentLabel(item,asset)}`}>{asset.kind==='document'?<span className="document-icon" aria-hidden="true">{asset.media_type==='application/pdf'?'PDF':'TXT'}</span>:<img src={`/api/v1/collections/${item.collection_id}/assets/${asset.id}/thumbnail`} loading="lazy" alt={asset.alt_text||`${asset.kind} thumbnail`}/>}<div className="asset-copy"><span className="copy-number">{asset.kind} · {asset.visibility==='curators'?'Curators only':'Shared'}</span><strong>{asset.alt_text||'No alternative text'}</strong><small>{attachmentLabel(item,asset)} · revision {asset.revision}</small>{asset.kind==='document'&&<DocumentActions collection={item.collection_id} asset={asset} onSessionExpired={onSessionExpired}/>}</div><div className="row-actions"><button disabled={busy} onClick={()=>setEditing(value=>!value)}>{editing?'Close editor':'Edit metadata'}</button><button disabled={busy} onClick={archive}>Archive {asset.kind==='document'?'document':'image'}</button></div>{editing&&<form className="ledger-form asset-metadata" aria-label={`Edit ${asset.kind} metadata`} onSubmit={event=>{event.preventDefault();void save();}}><div className="form-grid"><label>Visibility<select value={visibility} disabled={busy} onChange={event=>setVisibility(event.target.value as Visibility)}><option value="shared">Shared with viewers</option><option value="curators">Curators only</option></select></label><label>{asset.kind==='document'?'Document label':'Alternative text'}<input value={altText} maxLength={4096} disabled={busy} onChange={event=>setAltText(event.target.value)}/></label></div><div className="button-row"><button disabled={busy} type="button" onClick={()=>setEditing(false)}>Cancel</button><button className="primary" disabled={busy}>Save {asset.kind==='document'?'document':'image'} details</button></div></form>}</article>;
}

export default function AssetEditor({mode='images',onSessionExpired=()=>{},item,busy,setBusy,onSaved,onError}:Props){
 const headingId=useId();const fileRef=useRef<HTMLInputElement>(null);
 async function refresh(){onSaved(await api<Item>(`/items/${item.id}`));}
 async function changed(){await refresh();}
 return <section className="asset-editor" aria-labelledby={headingId}><div className="section-heading"><div><h3 id={headingId}>{mode==='documents'?'Documents':'Artwork and photos'}</h3><p className="muted">{mode==='documents'?'Save PDFs and UTF-8 text beside the record they describe. Documents start as Curators only; choose Shared to let Viewers read them.':'Save covers, photos, and scans beside the record they describe.'}</p></div></div><AssetUpload inputRef={fileRef} mode={mode} item={item} busy={busy} setBusy={setBusy} onSaved={onSaved} onError={onError}/><div className="asset-ledger" aria-live="polite">{item.assets.filter(asset=>(asset.kind==='document')===(mode==='documents')).length?item.assets.filter(asset=>(asset.kind==='document')===(mode==='documents')).map(asset=><AssetRow key={asset.id} item={item} asset={asset} busy={busy} setBusy={setBusy} onChanged={changed} onError={onError} restoreFocus={()=>fileRef.current?.focus()} onSessionExpired={onSessionExpired}/>):<p className="no-artwork"><strong>{mode==='documents'?'No documents attached.':'No artwork attached.'}</strong></p>}</div></section>;
}
