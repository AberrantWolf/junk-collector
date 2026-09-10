import React,{useEffect,useRef,useState,api} from '@collector/runtime';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type Asset=components['schemas']['PresentedAsset'];
type Visibility=components['schemas']['Visibility'];
type AssetKind=components['schemas']['AssetKind'];
type Props={item:Item;busy:boolean;setBusy:(value:boolean)=>void;onSaved:(item:Item)=>void;onError:(error:unknown)=>void};
type Target={value:string;kind:'item'|'specimen'|'component'|'capture'|'work'|'edition';id:string;label:string};

function targets(item:Item):Target[]{
 const values:Target[]=[{value:`item:${item.id}`,kind:'item',id:item.id,label:`Item · ${item.title}`}];
 if(item.identity_link.kind==='work')values.push({value:`work:${item.identity_link.work_id}`,kind:'work',id:item.identity_link.work_id,label:'Linked work'});
 if(item.identity_link.kind==='edition')values.push({value:`edition:${item.identity_link.edition_id}`,kind:'edition',id:item.identity_link.edition_id,label:'Linked edition'});
 for(const [copyIndex,specimen] of item.specimens.entries()){
  values.push({value:`specimen:${specimen.id}`,kind:'specimen',id:specimen.id,label:`Copy ${copyIndex+1} · ${specimen.label}`});
  for(const component of specimen.components)values.push({value:`component:${component.id}`,kind:'component',id:component.id,label:`Copy ${copyIndex+1} · Part ${component.position} · ${component.label}`});
 }
 for(const [index,capture] of item.captures.entries())values.push({value:`capture:${capture.id}`,kind:'capture',id:capture.id,label:`Capture ${index+1}`});
 return values;
}

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

function AssetRow({item,asset,busy,setBusy,onChanged,onError,restoreFocus}:{item:Item;asset:Asset;busy:boolean;setBusy:(value:boolean)=>void;onChanged:()=>Promise<void>;onError:(error:unknown)=>void;restoreFocus:()=>void}){
 const [editing,setEditing]=useState(false);const [visibility,setVisibility]=useState<Visibility>(asset.visibility||'shared');const [altText,setAltText]=useState(asset.alt_text);
 useEffect(()=>{setVisibility(asset.visibility||'shared');setAltText(asset.alt_text);},[asset.revision]);
 async function save(){setBusy(true);try{await api(`/collections/${item.collection_id}/assets/${asset.id}/metadata`,{method:'PUT',body:JSON.stringify({expected_revision:asset.revision,command:{visibility,alt_text:altText}})});await onChanged();setEditing(false);}catch(error){onError(error);}finally{setBusy(false);}}
 async function archive(){if(!confirm(`Archive this ${asset.kind}? The original remains preserved in revision history.`))return;let archived=false;setBusy(true);try{await api(`/collections/${item.collection_id}/assets/${asset.id}/archive?expected_revision=${asset.revision}`,{method:'POST'});await onChanged();archived=true;}catch(error){onError(error);}finally{setBusy(false);if(archived)requestAnimationFrame(restoreFocus);}}
 return <article className="asset-record" aria-label={`${asset.kind} attached to ${attachmentLabel(item,asset)}`}><img src={`/api/v1/collections/${item.collection_id}/assets/${asset.id}/thumbnail`} loading="lazy" alt={asset.alt_text||`${asset.kind} thumbnail`}/><div className="asset-copy"><span className="copy-number">{asset.kind} · {asset.visibility==='curators'?'Curators only':'Shared'}</span><strong>{asset.alt_text||'No alternative text'}</strong><small>{attachmentLabel(item,asset)} · revision {asset.revision}</small></div><div className="row-actions"><button disabled={busy} onClick={()=>setEditing(value=>!value)}>{editing?'Close editor':'Edit metadata'}</button><button disabled={busy} onClick={archive}>Archive image</button></div>{editing&&<form className="ledger-form asset-metadata" aria-label={`Edit ${asset.kind} metadata`} onSubmit={event=>{event.preventDefault();void save();}}><div className="form-grid"><label>Visibility<select value={visibility} disabled={busy} onChange={event=>setVisibility(event.target.value as Visibility)}><option value="shared">Shared with viewers</option><option value="curators">Curators only</option></select></label><label>Alternative text<input value={altText} maxLength={4096} disabled={busy} onChange={event=>setAltText(event.target.value)}/></label></div><div className="button-row"><button disabled={busy} type="button" onClick={()=>setEditing(false)}>Cancel</button><button className="primary" disabled={busy}>Save metadata</button></div></form>}</article>;
}

export default function AssetEditor({item,busy,setBusy,onSaved,onError}:Props){
 const availableTargets=targets(item);const [target,setTarget]=useState(availableTargets[0]?.value||'');const [kind,setKind]=useState<AssetKind>('cover');const [visibility,setVisibility]=useState<Visibility>('shared');const [altText,setAltText]=useState('');const [file,setFile]=useState<File>();const fileRef=useRef<HTMLInputElement>(null);const [preview,setPreview]=useState('');
 useEffect(()=>{if(!file){setPreview('');return;}const url=URL.createObjectURL(file);setPreview(url);return()=>URL.revokeObjectURL(url);},[file]);
 useEffect(()=>{if(!availableTargets.some(value=>value.value===target))setTarget(availableTargets[0]?.value||'');},[item.revision]);
 async function refresh(){onSaved(await api<Item>(`/items/${item.id}`));}
 async function attach(){const selected=availableTargets.find(value=>value.value===target);if(!file||!selected)return;setBusy(true);try{const query=new URLSearchParams({subject_kind:selected.kind,subject_id:selected.id,kind,visibility});if(altText)query.set('alt_text',altText);await api(`/collections/${item.collection_id}/assets?${query}`,{method:'POST',body:file});await refresh();setFile(undefined);setAltText('');if(fileRef.current)fileRef.current.value='';}catch(error){onError(error);}finally{setBusy(false);}}
 async function changed(){await refresh();}
 return <section className="asset-editor" aria-labelledby="asset-editor-title"><div className="section-heading"><div><h3 id="asset-editor-title">Supporting images</h3><p className="muted">Preserve covers, photos, and scans beside the record they describe.</p></div></div><form className="ledger-form" aria-label="Attach supporting image" onSubmit={event=>{event.preventDefault();void attach();}}><div className="form-grid"><label>Attachment<select value={target} disabled={busy} onChange={event=>setTarget(event.target.value)}>{availableTargets.map(value=><option value={value.value} key={value.value}>{value.label}</option>)}</select></label><label>Image role<select value={kind} disabled={busy} onChange={event=>setKind(event.target.value as AssetKind)}><option value="cover">Cover</option><option value="photo">Photo</option><option value="scan">Scan</option></select></label><label>Visibility<select value={visibility} disabled={busy} onChange={event=>setVisibility(event.target.value as Visibility)}><option value="shared">Shared with viewers</option><option value="curators">Curators only</option></select></label><label>Alternative text<input value={altText} maxLength={4096} disabled={busy} placeholder="Describe the image" onChange={event=>setAltText(event.target.value)}/></label></div><label>Artwork image<input ref={fileRef} type="file" disabled={busy} accept="image/png,image/jpeg,image/webp" onChange={event=>setFile(event.target.files?.[0])}/></label>{preview&&<div className="asset-preview"><img src={preview} alt="Selected image preview"/><span><strong>{file?.name}</strong><small>Preview before preservation</small></span></div>}<div className="button-row"><button className="primary" disabled={busy||!file||!target}>Attach image</button></div></form><div className="asset-ledger" aria-live="polite">{item.assets.length?item.assets.map(asset=><AssetRow key={asset.id} item={item} asset={asset} busy={busy} setBusy={setBusy} onChanged={changed} onError={onError} restoreFocus={()=>fileRef.current?.focus()}/>):<p className="no-artwork"><strong>No artwork attached.</strong><span>This is separate from whether the catalog identity is unidentified or tentative.</span></p>}</div></section>;
}
