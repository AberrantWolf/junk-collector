import React,{useEffect,useRef,useState,api,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import GuidedIngest from './guided-ingest';
import {uploadPackage} from './package-upload';
type Item=components['schemas']['PresentedItem'];
type Plan=components['schemas']['IngestPlan'];
type Options=components['schemas']['CleanReimportOptions'];
export default function CleanReimport({item,busy,setBusy,onPublished,onError}:{item:Item;busy:boolean;setBusy:(value:boolean)=>void;onPublished:(item:Item)=>void;onError:(error:unknown)=>void}){
 const [capture,setCapture]=useState(item.captures[0]?.id||'');const [source,setSource]=useState<'retained'|'upload'>('retained');const [files,setFiles]=useState<File[]>([]);
 const [resetMetadata,setResetMetadata]=useState(false);const [resetArtwork,setResetArtwork]=useState(false);const [automatic,setAutomatic]=useState(true);
 const [review,setReview]=useState<{plan:Plan;options:Options;revision:number;files:File[];key:string;automatic:boolean}>();const [error,setError]=useState('');const [importId,setImportId]=useState(item.reimport_import_id||'');
 const lifecycle=useRef(new AbortController());useEffect(()=>{lifecycle.current=new AbortController();return()=>lifecycle.current.abort();},[]);
 const original=item.captures.find(value=>value.id===capture)?.representations.find(value=>value.role==='preservation_master');
 async function preview(){setBusy(true);setError('');try{
  if(!capture)throw new Error('Choose the capture to replace.');if(source==='retained'&&!original)throw new Error('This capture has no retained original package. Choose a replacement upload.');if(source==='upload'&&!files.length)throw new Error('Choose the complete replacement package.');
  const signal=lifecycle.current.signal;
  const metadata=await api<components['schemas']['MetadataReviewView']>(`/items/${item.id}/metadata`,{signal});
  const plan=await api<Plan>('/ingests/plan',{method:'POST',signal,body:JSON.stringify({collection_id:item.collection_id,packages:[{source:{kind:'browser',label:null,package_name:source==='retained'?'Retained original':files[0].name},files:source==='retained'?original!.files.map(file=>({name:file.name,bytes:file.bytes,sha256:file.digest.hex})):files.map(file=>({name:file.webkitRelativePath||file.name,bytes:file.size,sha256:null})),anchor_file_index:null}]})});
  const canAnalyze=plan.packages.every(pkg=>pkg.analysis_available);if(!canAnalyze)setAutomatic(false);
  setReview({plan,options:{replace_captures:[capture],retained_representation:source==='retained'?original!.id:null,reset_metadata:resetMetadata,reset_artwork:resetArtwork,expected_metadata_revision:metadata.revision},revision:item.revision,files:[...files],key:crypto.randomUUID(),automatic:automatic&&canAnalyze});
 }catch(error){setError(requestErrorMessage(error));onError(error);}finally{setBusy(false);}}
 async function start(){if(!review)return;setBusy(true);setError('');try{
  const signal=lifecycle.current.signal;
  const accepted=await api<components['schemas']['AcceptedIngest']>('/ingests',{method:'POST',signal,headers:{'Idempotency-Key':review.key},body:JSON.stringify({plan:review.plan,destination:{item_id:item.id,expected_revision:review.revision},clean_reimport:review.options,analysis:review.automatic?'automatic':'preserve_only',enrichment:review.automatic?'automatic':'none',prepare_music:false})});
  const id=accepted.imports[0]?.import_id;if(!id)throw new Error('No reimport was accepted.');setImportId(id);
  const current=await api<Item>(`/items/${item.id}`,{signal});signal.throwIfAborted();onPublished(current);
  const detail=await api<components['schemas']['IngestDetail']>(`/ingests/${id}`,{signal});
  if(!review.options.retained_representation&&detail.import.phase==='staging')await uploadPackage(detail.import,review.files,false,signal,()=>{},()=>{});
  setReview(undefined);setFiles([]);
 }catch(error){setError(requestErrorMessage(error));onError(error);}finally{setBusy(false);}}
 return <section className="guided-ingest" aria-label="Replace original files" data-unsaved={!importId&&(!!review||!!files.length)} data-uploading={busy&&!!importId&&!!files.length}><h3>Replace original files</h3><p>Use this when the preserved dump needs replacing. {item.domain.module==='org.junk.music'&&<>For missing album or track names, use Music → Find album and track names instead. </>} The current capture stays playable until its replacement succeeds; copies, loans, earlier originals and evidence remain.</p>
 {importId?<><GuidedIngest key={importId} historyOnly initialImportId={importId} collectionId={item.collection_id} busy={busy} setBusy={setBusy} onPublished={onPublished} onError={onError}/><button disabled={busy} onClick={()=>{setImportId('');setCapture(item.captures[0]?.id||'');setReview(undefined);setError('');}}>Plan another replacement</button></>:<>
 <fieldset disabled={busy||!!review}><legend>Replacement</legend><label>Original import to replace<select value={capture} onChange={event=>setCapture(event.target.value)}>{item.captures.map((value,index)=><option key={value.id} value={value.id}>{index+1}. {value.representations.find(rep=>rep.role==='preservation_master')?.files[0]?.name||value.id}</option>)}</select></label><label>Package source<select value={source} onChange={event=>setSource(event.target.value as typeof source)}><option value="retained">Use saved original files (no upload)</option><option value="upload">Upload a replacement package</option></select></label>{source==='upload'&&<label>Replacement package files<input type="file" multiple onChange={event=>setFiles(Array.from(event.target.files||[]))}/></label>}
 <label><input type="checkbox" checked={automatic} onChange={event=>setAutomatic(event.target.checked)}/>Analyze and look up metadata automatically</label><label><input type="checkbox" checked={resetMetadata} onChange={event=>setResetMetadata(event.target.checked)}/>Clear my saved metadata</label><label><input type="checkbox" checked={resetArtwork} onChange={event=>setResetArtwork(event.target.checked)}/>Clear my artwork choice</label></fieldset>
 {!review?<button disabled={busy||!capture} onClick={()=>void preview()}>Review replacement</button>:<section aria-label="Review replacement"><h4>What will change</h4><p>Unfinished work for the replaced import will stop and leave your task history. Tasks also needed elsewhere will continue. Automatic metadata starts fresh. {review.options.reset_metadata?'Manual metadata overrides will reset when this starts.':'Manual metadata overrides will be preserved.'} {review.options.reset_artwork?'The selected artwork override will reset when this starts.':'Selected artwork will be preserved.'} Inventory, copies, loans and the item identity stay unchanged.</p><p>{review.options.retained_representation?'Retained bytes will be read and checked against their recorded hashes in background work.':'The complete replacement package will be uploaded.'} Failed replacement keeps the previous usable capture.</p>{review.plan.packages.flatMap(pkg=>pkg.blockers).map((blocker,index)=><p key={index}>{blocker.message}</p>)}<button disabled={busy} onClick={()=>void start()}>Start replacement</button><button disabled={busy} onClick={()=>setReview(undefined)}>Back to options</button></section>}
 </>}{error&&<p role="alert">{error}</p>}</section>;
}
