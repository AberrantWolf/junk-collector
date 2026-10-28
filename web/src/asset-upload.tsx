import React,{useEffect,useRef,useState,api,useOperationScope,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
type Item=components['schemas']['PresentedItem'];
type Asset=components['schemas']['PresentedAsset'];
type Visibility=components['schemas']['Visibility'];
type AssetKind=components['schemas']['AssetKind'];
type Props={mode?:'images'|'documents';coverOnly?:boolean;inputRef?:React.RefObject<HTMLInputElement|null>;item:Item;busy:boolean;setBusy:(value:boolean)=>void;onSaved:(item:Item,asset:Asset)=>void;onError:(error:unknown)=>void};
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

export default function AssetUpload({mode='images',coverOnly=false,inputRef,item,busy,setBusy,onSaved,onError}:Props){
 const availableTargets=targets(item);const [target,setTarget]=useState(availableTargets[0]?.value||'');const [kind,setKind]=useState<AssetKind>(mode==='documents'?'document':'cover');const [visibility,setVisibility]=useState<Visibility>(mode==='documents'?'curators':'shared');const [altText,setAltText]=useState('');const [file,setFile]=useState<File>();const localFileRef=useRef<HTMLInputElement>(null);const fileRef=inputRef??localFileRef;const [preview,setPreview]=useState('');const [pending,setPending]=useState<{query:string;file:File;key:string}>();
 useEffect(()=>{if(!file||mode==='documents'){setPreview('');return;}const url=URL.createObjectURL(file);setPreview(url);return()=>URL.revokeObjectURL(url);},[file]);
 useEffect(()=>{if(!availableTargets.some(value=>value.value===target))setTarget(availableTargets[0]?.value||'');},[item.revision]);
 const upload=useOperationScope(item.id);
 const [error,setError]=useState('');
 async function attach(){const selected=coverOnly?{kind:'item',id:item.id}:availableTargets.find(value=>value.value===target);if(!file||!selected)return;setBusy(true);setError('');
  await upload.run(async attempt=>{
   let submission=pending;
   if(!submission){const query=new URLSearchParams({subject_kind:selected.kind,subject_id:selected.id,kind:coverOnly?'cover':kind,visibility});if(altText||mode==='documents')query.set('alt_text',altText||file.name);submission={query:query.toString(),file,key:crypto.randomUUID()};setPending(submission);}
   const asset=await attempt.wait(api<Asset>(`/collections/${item.collection_id}/assets?${submission.query}`,{method:'POST',headers:{'Idempotency-Key':submission.key},body:submission.file,signal:attempt.signal}));
   const refreshed=await attempt.wait(api<Item>(`/items/${item.id}`,{signal:attempt.signal}));
   return {asset,item:refreshed};
  },{success:result=>{onSaved(result.item,result.asset);setPending(undefined);setFile(undefined);setAltText('');if(fileRef.current)fileRef.current.value='';},failure:error=>{if(isApiRequestError(error)&&[400,403,404,409,413,422].includes(error.status))setPending(undefined);setError(requestErrorMessage(error));onError(error);},settled:()=>setBusy(false)});
 }
 return <form data-unsaved={!!file||!!altText||!!pending} className={`ledger-form asset-upload${coverOnly?' cover-upload':''}`} aria-label={coverOnly?'Upload manual cover art':mode==='documents'?'Attach supporting document':'Attach supporting image'} onSubmit={event=>{event.preventDefault();void attach();}}><div className="form-grid">{!coverOnly&&<label>Attachment<select value={target} disabled={busy||!!pending} onChange={event=>setTarget(event.target.value)}>{availableTargets.map(value=><option value={value.value} key={value.value}>{value.label}</option>)}</select></label>}{!coverOnly&&mode!=='documents'&&<label>Image role<select value={kind} disabled={busy||!!pending} onChange={event=>setKind(event.target.value as AssetKind)}><option value="cover">Cover</option><option value="photo">Photo</option><option value="scan">Scan</option></select></label>}<label>Visibility<select value={visibility} disabled={busy||!!pending} onChange={event=>setVisibility(event.target.value as Visibility)}><option value="shared">Shared with viewers</option><option value="curators">Curators only</option></select></label><label>{mode==='documents'?'Document label':'Alternative text'}<input value={altText} maxLength={4096} disabled={busy||!!pending} placeholder={mode==='documents'?'Manual, booklet, or receipt':'Describe the image'} onChange={event=>setAltText(event.target.value)}/></label></div><label>{coverOnly?'Cover image':mode==='documents'?'Document file':'Artwork image'}<input ref={fileRef} type="file" disabled={busy||!!pending} accept={mode==='documents'?'.pdf,.txt,application/pdf,text/plain':'image/png,image/jpeg,image/webp'} onChange={event=>setFile(event.target.files?.[0])}/></label><small>{mode==='documents'?'PDF or UTF-8 text':'PNG, JPEG or WebP'}, up to 20 MiB. The original is preserved.</small>{preview&&<div className="asset-preview"><img src={preview} alt="Selected image preview"/><span><strong>{file?.name}</strong><small>Image preview</small></span></div>}{pending&&!busy&&<p className="notice">The previous upload is unconfirmed. Retry to resolve it before changing the attachment.</p>}{error&&<p className="notice error" role="alert">{error}</p>}<div className="button-row"><button className="primary" disabled={busy||!file||!target}>{pending?'Retry attachment':coverOnly?'Upload cover':mode==='documents'?'Attach document':'Attach image'}</button></div></form>;
}
