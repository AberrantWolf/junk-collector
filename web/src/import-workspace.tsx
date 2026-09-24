import {uploadPackage,type UploadProgress} from './package-upload';
import {WorkProgress,UploadMeter} from './work-progress';
import React,{useEffect,useState,useRef,api,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type ImportView=components['schemas']['ImportView'];
type InboxSource=components['schemas']['InboxSourceView'];
type JobView=components['schemas']['JobView'];
type Association=components['schemas']['CaptureAssociation'];
type Props={initialImportId?:string;item:Item;onPublished:(item:Item)=>void;onError:(error:unknown)=>void};

const terminal=new Set<JobView['state']>(['completed','failed','cancelled']);
const phaseLabel:Record<ImportView['phase'],string>={staging:'Uploading files',analyzing:'Upload complete · analysis needed',awaiting_review:'Analysis complete · review needed',publishing:'Publishing capture',published:'Capture published',cancelled:'Import cancelled'};

function fileName(file:File){return file.webkitRelativePath||file.name;}
function packageName(files:File[]){const first=files[0]&&fileName(files[0]);return first?.includes('/')?first.split('/')[0]:'Browser package';}
function percent(done:number,total:number){return total?Math.min(100,Math.round(done/total*100)):100;}
function jobsChanged(){window.dispatchEvent(new Event('collector-jobs-updated'));}

async function waitForJob(id:string,onProgress:(job:JobView)=>void,signal:AbortSignal){
 for(;;){
  signal.throwIfAborted();const job=await api<JobView>(`/jobs/${id}`,{signal});signal.throwIfAborted();onProgress(job);
  if(terminal.has(job.state)){
   if(job.state==='failed')throw new Error(job.error||job.limitation||'The background job failed.');
   if(job.state==='cancelled')throw new Error('The background job was cancelled.');
   return job;
  }
  await new Promise(resolve=>setTimeout(resolve,2000));
 }
}

export default function ImportWorkspace({initialImportId,item,onPublished,onError}:Props){
 const [busy,setBusy]=useState(false);const [analysisError,setAnalysisError]=useState('');const [analysisPollRevision,setAnalysisPollRevision]=useState(0);
 const lifecycle=useRef(new AbortController());
 const [uploadProgress,setUploadProgress]=useState<UploadProgress>();const [activity,setActivity]=useState('');
 async function request<T=unknown>(path:string,init:RequestInit={}):Promise<T>{const signal=lifecycle.current.signal;signal.throwIfAborted();const value=await api<T>(path,{...init,signal});signal.throwIfAborted();return value;}
 function report(error:unknown){if(!lifecycle.current.signal.aborted)onError(error);}
 function finish(){if(!lifecycle.current.signal.aborted){setBusy(false);setUploadProgress(undefined);setActivity('');}}
 useEffect(()=>{if(!uploadProgress)return;const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue='';};window.addEventListener('beforeunload',warn);return()=>window.removeEventListener('beforeunload',warn);},[!!uploadProgress]);
 useEffect(()=>{lifecycle.current=new AbortController();return()=>{lifecycle.current.abort();setBusy(false);};},[item.id]);
 const [source,setSource]=useState<'browser'|'inbox'>('browser');
 const [files,setFiles]=useState<File[]>([]);const [inboxes,setInboxes]=useState<InboxSource[]>([]);
 const [label,setLabel]=useState('');const [inboxPackage,setInboxPackage]=useState('');
 const [record,setRecord]=useState<ImportView>();const [job,setJob]=useState<JobView>();
 const [editingReview,setEditingReview]=useState(false);
 const checkingFiles=uploadProgress?.phase==='checking';
 const [conflict,setConflict]=useState('');const [itemRevision,setItemRevision]=useState(item.revision);
 const [identity,setIdentity]=useState('unidentified');const [attach,setAttach]=useState(true);
 const [specimen,setSpecimen]=useState('');const [component,setComponent]=useState('');const [carrier,setCarrier]=useState('');
 const selectedSpecimen=item.specimens.find(value=>value.id===specimen);
 const selectedComponent=selectedSpecimen?.components.find(value=>value.id===component);
 const total=record?.files.reduce((sum,file)=>sum+file.bytes,0)||files.reduce((sum,file)=>sum+file.size,0);
 const done=record?.files.reduce((sum,file)=>sum+file.received_bytes,0)||0;
 const candidates=record?.candidates||[];
 const selectedInbox=inboxes.find(value=>value.label===label);
 const storageKey=`collector.import.${item.id}`;

 useEffect(()=>{let active=true;request<InboxSource[]>(`/collections/${item.collection_id}/inboxes`).then(value=>{if(!active)return;setInboxes(value);setLabel(current=>current||value[0]?.label||'');}).catch(report);return()=>{active=false;};},[item.collection_id]);
 useEffect(()=>{const id=initialImportId||localStorage.getItem(storageKey);if(id)request<ImportView>(`/imports/${id}`).then(setRecord).catch(error=>{if(isApiRequestError(error)&&(error.status===404||error.status===403))localStorage.removeItem(storageKey);report(error);});},[storageKey,initialImportId]);
 useEffect(()=>{if(record)localStorage.setItem(storageKey,record.id);},[record?.id,storageKey]);
 useEffect(()=>{setItemRevision(item.revision);},[item.revision]);
 useEffect(()=>{setInboxPackage(selectedInbox?.packages[0]||'');},[selectedInbox?.label]);
 useEffect(()=>{const refresh=()=>{if(record)request<ImportView>(`/imports/${record.id}`).then(setRecord).catch(report);};window.addEventListener('online',refresh);window.addEventListener('collector-jobs-updated',refresh);return()=>{window.removeEventListener('online',refresh);window.removeEventListener('collector-jobs-updated',refresh);};},[record?.id]);

 function reset(){setEditingReview(false);localStorage.removeItem(storageKey);setRecord(undefined);setJob(undefined);setConflict('');setFiles([]);setIdentity('unidentified');setAttach(true);setSpecimen('');setComponent('');setCarrier('');}
 async function stage(){
  setBusy(true);setActivity('Preparing upload…');setJob(undefined);
  try{
   if(source==='browser'){
    const created=await request<ImportView>('/imports',{method:'POST',body:JSON.stringify({collection_id:item.collection_id,package_name:packageName(files),files:files.map(file=>({name:fileName(file),bytes:file.size,sha256:null}))})});
    setRecord(created);
    await uploadPackage(created,files,false,lifecycle.current.signal,setUploadProgress,setRecord);
   }else{
    const staged=await request<{import:ImportView;job_id:string}>(`/collections/${item.collection_id}/inboxes/${encodeURIComponent(label)}/stage`,{method:'POST',body:JSON.stringify({package:inboxPackage})});
    setRecord(staged.import);jobsChanged();await waitForJob(staged.job_id,setJob,lifecycle.current.signal);jobsChanged();setRecord(await request<ImportView>(`/imports/${staged.import.id}`));
   }
  }catch(error){report(error);}finally{finish();}
 }
 async function resume(){
  if(!record)return;setBusy(true);setActivity('Checking retained upload…');
  try{
   const current=await request<ImportView>(`/imports/${record.id}`);setRecord(current);
   if(current.phase!=='staging')return;
   await uploadPackage(current,files,true,lifecycle.current.signal,setUploadProgress,setRecord);
  }catch(error){report(error);}finally{finish();}
 }
 async function editReview(){
  if(!record)return;setBusy(true);
  try{
   const currentItem=await request<Item>(`/items/${item.id}`);setItemRevision(currentItem.revision);
   const decision=record.review;
   setIdentity(decision?.identity.identity==='candidate'?decision.identity.key:'unidentified');
   setAttach(!!record.association.item_id);setSpecimen(record.association.specimen_id||'');setComponent(record.association.component_id||'');setCarrier(record.association.carrier_id||'');
   setEditingReview(true);
  }catch(error){report(error);}finally{finish();}
 }
 useEffect(()=>{
  if(record?.phase!=='analyzing'||!record.analysis_job_id)return;
  const controller=new AbortController();const id=record.id;setJob(undefined);setAnalysisError('');
  void waitForJob(record.analysis_job_id,value=>{if(!controller.signal.aborted)setJob(value);},controller.signal).then(async()=>{const value=await api<ImportView>(`/imports/${id}`,{signal:controller.signal});if(!controller.signal.aborted){setRecord(value);jobsChanged();}}).catch(error=>{if(!controller.signal.aborted)setAnalysisError(error instanceof Error?error.message:String(error));});
  return()=>controller.abort();
 },[record?.id,record?.phase,record?.analysis_job_id,analysisPollRevision]);
 async function beginReview(){
  if(!record)return;setBusy(true);
  try{const accepted=await request<{job_id:string}>(`/imports/${record.id}/analysis`,{method:'POST',body:JSON.stringify({module:item.domain.module})});setRecord({...record,analysis_job_id:accepted.job_id});jobsChanged();}
  catch(error){report(error);}finally{finish();}
 }
 function association():Association{
  if(!attach)return {item_id:null,specimen_id:null,component_id:null,carrier_id:null};
  return {item_id:item.id,specimen_id:specimen||null,component_id:component||null,carrier_id:carrier||null};
 }
 async function saveReview(){
  if(!record)return;setBusy(true);
  try{
   const choice=identity==='unidentified'?{identity:'unidentified'}:{identity:'candidate',key:identity};
   const reviewed=await request<ImportView>(`/imports/${record.id}/review`,{method:'PUT',body:JSON.stringify({expected_revision:record.revision,identity:choice,association:association(),expected_item_revision:attach?itemRevision:null})});setRecord(reviewed);setConflict('');setEditingReview(false);
  }catch(error){
   if(isApiRequestError(error)&&error.status===409){
    const [current,currentItem]=await Promise.all([request<ImportView>(`/imports/${record.id}`),request<Item>(`/items/${item.id}`)]);setRecord(current);setItemRevision(currentItem.revision);setConflict(`The server is now at Import revision ${current.revision} and Item revision ${currentItem.revision}. Your review choices are still in this form; review them and apply again.`);
   }else report(error);
  }finally{finish();}
 }
 async function publish(){
  if(!record)return;setBusy(true);setActivity('Publishing capture…');setJob(undefined);
  try{
   const accepted=await request<{job_id:string}>(`/imports/${record.id}/publish`,{method:'POST'});
   jobsChanged();await waitForJob(accepted.job_id,setJob,lifecycle.current.signal);jobsChanged();setRecord(await request<ImportView>(`/imports/${record.id}`));onPublished(await request<Item>(`/items/${item.id}`));
  }catch(error){report(error);try{setRecord(await request<ImportView>(`/imports/${record.id}`));}catch(refreshError){report(refreshError);}}finally{finish();}
 }

 const canStage=source==='browser'?files.length>0:!!label&&!!inboxPackage;
 return <section data-uploading={!!uploadProgress} data-unsaved={(!record&&files.length>0)||editingReview||(record?.phase==='awaiting_review'&&!record.review&&(identity!=='unidentified'||!attach||!!specimen||!!component||!!carrier))} className="import-workspace" aria-labelledby="import-heading">
  <div className="section-heading"><div><h3 id="import-heading">Capture accession</h3><p className="muted">Stage original package bytes, inspect the record, then choose what—if anything—they belong to.</p></div>{record&&!initialImportId&&<button disabled={busy} onClick={reset}>Stage another package</button>}</div>
  {initialImportId&&!record&&<p role="status">Loading saved import…</p>}
  {!record&&!initialImportId&&<form aria-label="Stage package" onSubmit={event=>{event.preventDefault();void stage();}}>
   <fieldset className="source-choice"><legend>Package source</legend><label><input type="radio" name="import-source" checked={source==='browser'} onChange={()=>setSource('browser')}/> This browser</label><label><input type="radio" name="import-source" checked={source==='inbox'} onChange={()=>setSource('inbox')}/> Configured inbox</label></fieldset>
   {source==='browser'?<label>Original package files<input aria-label="Original package files" type="file" disabled={busy} multiple onChange={event=>setFiles(Array.from(event.target.files||[]))}/></label>:inboxes.length?<div className="form-grid"><label>Inbox<select value={label} disabled={busy} onChange={event=>setLabel(event.target.value)}>{inboxes.map(value=><option key={value.label}>{value.label}</option>)}</select></label><label>Package<select value={inboxPackage} disabled={busy} onChange={event=>setInboxPackage(event.target.value)}>{selectedInbox?.packages.map(value=><option key={value}>{value}</option>)}</select></label></div>:<p className="notice">No configured inbox packages are available. Choose files from this browser.</p>}
   {source==='browser'&&files.length>0&&<div className="package-files compact" role="list">{files.map(file=><div role="listitem" key={fileName(file)}><span>{fileName(file)}</span><small>{file.size.toLocaleString()} bytes</small></div>)}</div>}
   <button className="primary" disabled={busy||!canStage}>Stage package</button>
  </form>}
  {uploadProgress?<UploadMeter value={uploadProgress}/>:job&&!terminal.has(job.state)?<WorkProgress label={job.phase.replaceAll('_',' ')} completed={job.total_units?job.completed_units:undefined} total={job.total_units} detail="Server work continues independently of this window."/>:activity&&<WorkProgress label={activity}/>}
  {record&&<div className="import-docket">
   <header><div><span className="docket-phase">{record.phase==='awaiting_review'&&record.review?'Review saved · publication needed':phaseLabel[record.phase]}</span><strong className="docket-title">{record.source.package_name}</strong></div><span>Upload {percent(done,total)}%</span></header>
   {record.phase==='analyzing'&&<p className="notice">{record.analysis_job_id&&(!job||!terminal.has(job.state))?'Package analysis is underway. You can leave this panel and return to its progress.':'Your files are uploaded, but are not attached to the album yet. Choose Review import to analyze the package.'}</p>}
   {record.phase==='analyzing'&&analysisError&&<p role="alert">{analysisError} <button onClick={()=>setAnalysisPollRevision(value=>value+1)}>Refresh analysis status</button></p>}
   {record.phase==='awaiting_review'&&!record.review&&<p className="notice">Analysis is complete. Confirm the album attachment and save your review below, then choose Publish capture. Tracks become available for preparation after publication.</p>}
   {record.phase==='awaiting_review'&&record.review&&<p className="notice">Your review is saved. Choose Publish capture to preserve this package and attach it to the selected album.</p>}
   <progress max={Math.max(total,1)} value={total?done:1} aria-label="Package staging progress"/><p className="muted" role="status" aria-live="polite">{checkingFiles?'Checking selected files against retained bytes':job?`${job.phase}: ${percent(job.completed_units,job.total_units)}%`:`${done.toLocaleString()} of ${total.toLocaleString()} bytes staged`}</p>
   <h4>Package inventory</h4><div className="package-files" role="list">{record.files.map(file=><div role="listitem" key={file.name}><span>{file.name}</span><small>{file.received_bytes.toLocaleString()} / {file.bytes.toLocaleString()} bytes · {file.digest?'SHA-256 recorded':'digest pending'}</small></div>)}</div>
   {record.limitations.length>0&&<div className="import-limitations"><h4>Analysis limitations</h4><ul>{record.limitations.map(value=><li key={value}>{value}</li>)}</ul></div>}
   {!uploadProgress&&record.phase==='staging'&&record.source.kind==='browser'&&<form aria-label="Resume package" onSubmit={event=>{event.preventDefault();void resume();}}>
    <p className="notice">Upload interrupted? Select the same original package files to continue. Retained bytes are checked before uploading the remainder.</p>
    <label>Original package files<input aria-label="Original package files" type="file" disabled={busy} multiple onChange={event=>setFiles(Array.from(event.target.files||[]))}/></label>
    <button className="primary" disabled={busy||!files.length}>Resume upload</button>
   </form>}
   {record.phase==='analyzing' &&<button className="primary" disabled={busy||!!record.analysis_job_id&&(!job||!terminal.has(job.state))} onClick={()=>void beginReview()}>{record.analysis_job_id&&(!job||!terminal.has(job.state))?'Analysis in progress…':'Review import'}</button>}
   {conflict&&<p className="notice conflict" role="alert"><strong>Revision conflict.</strong> {conflict}</p>}
   {record.phase==='awaiting_review'&&(!record.review||!!conflict||editingReview)&&<form aria-label="Review import" onSubmit={event=>{event.preventDefault();void saveReview();}}>
    <fieldset><legend>Identity</legend><label><input type="radio" name="identity" value="unidentified" checked={identity==='unidentified'} onChange={event=>setIdentity(event.target.value)}/> Keep unidentified</label>{candidates.map(candidate=><label key={candidate.key}><input type="radio" name="identity" value={candidate.key} checked={identity===candidate.key} onChange={event=>setIdentity(event.target.value)}/> {candidate.label}</label>)}</fieldset>
    <fieldset><legend>Physical association</legend><label><input type="radio" name="attachment" checked={attach} onChange={()=>setAttach(true)}/> Attach to {item.title}</label><label><input type="radio" name="attachment" checked={!attach} onChange={()=>setAttach(false)}/> Leave capture unattached</label></fieldset>
    {attach&&item.specimens.length>0&&<div className="form-grid"><label>Physical copy<select value={specimen} onChange={event=>{setSpecimen(event.target.value);setComponent('');setCarrier('');}}><option value="">Item only</option>{item.specimens.map(value=><option key={value.id} value={value.id}>{value.label}</option>)}</select></label>{selectedSpecimen&&<label>Part<select value={component} onChange={event=>{setComponent(event.target.value);setCarrier('');}}><option value="">Copy only</option>{selectedSpecimen.components.map(value=><option key={value.id} value={value.id}>{value.position}. {value.label}</option>)}</select></label>}{selectedComponent?.carrier&&<label>Carrier<select value={carrier} onChange={event=>setCarrier(event.target.value)}><option value="">Part only</option><option value={selectedComponent.carrier.id}>{selectedComponent.carrier.label}</option></select></label>}</div>}
    <button className="primary" disabled={busy}>Save review</button>
   </form>}
   {record.phase==='awaiting_review'&&record.review&&!conflict&&!editingReview&&<div className="review-decision"><p><strong>Review saved.</strong> {record.review.identity.identity==='unidentified'?'No catalog identity was asserted.':'A candidate was selected for this capture.'}</p><button disabled={busy} onClick={()=>void editReview()}>Edit review</button><button className="primary" disabled={busy} onClick={()=>void publish()}>Publish capture</button></div>}
   {record.phase==='published'&&<p className="notice" role="status">Capture published. The original package and integrity evidence are now in the archive.</p>}
  </div>}
 </section>;
}
