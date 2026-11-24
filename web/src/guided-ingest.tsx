import {loadDraft,saveDraft} from './intake-drafts';
import IngestStatus from './ingest-status';
import {presentationLabel} from './presentation-labels';
import {uploadPackage,type UploadProgress} from './package-upload';
import {WorkProgress,UploadMeter,bytes} from './work-progress';
import React,{useEffect,useRef,useState,api,getApiIdentity,useOperationScope,isApiRequestError,requestErrorMessage,retainUnchanged} from '@collector/runtime';
import type {components} from './generated/api';
import DestinationChooser from './destination-chooser';
type Detail=components['schemas']['IngestDetail'];
type Item=components['schemas']['PresentedItem'];
// One import's progress, actions and destination. New imports start in Add to
// library; this shows them wherever an import is opened (Review, Activity,
// Replace files).
type Props={initialImportId:string;collectionId:string;busy:boolean;setBusy:(value:boolean)=>void;onPublished:(item:Item,replacedItemId?:string)=>void;onError:(error:unknown)=>void};
function fileName(file:File){return file.webkitRelativePath||file.name;}
function stateLabel(value:string){return presentationLabel(value);}

export default function GuidedIngest({initialImportId,collectionId,busy,setBusy,onPublished,onError}:Props){
 const [resumeFiles,setResumeFiles]=useState<File[]>([]);
 const [detail,setDetail]=useState<Detail>();const [detailOpen,setDetailOpen]=useState(false);const [target,setTarget]=useState<Item|null>();
 const [uploadProgress,setUploadProgress]=useState<UploadProgress>();const [activity,setActivity]=useState('');
 const [inboxConfirmed,setInboxConfirmed]=useState(false);
 useEffect(()=>setInboxConfirmed(false),[detail?.import.id]);
 const [notice,setNotice]=useState('');const [failure,setFailure]=useState('');
 const lifecycle=useRef(new AbortController());const reads=useOperationScope(collectionId);const reading=useRef(false);const mutating=useRef(false);
const detailId=useRef<string|undefined>(undefined);
 // Under "All collections" the page has none, so the import's own is used.
 const destinationCollection=detail?.summary.collection_id||collectionId;
 const targetEdited=useRef(false);
 const associationChanged=!!detail&&target!==undefined&&(target?.id??null)!==(detail.import.association.item_id??null);
 const callbacks=useRef({onPublished,onError,setBusy});callbacks.current={onPublished,onError,setBusy};
 const notified=useRef(new Map<string,string>());
 async function request<T=unknown>(path:string,init:RequestInit={}):Promise<T>{
  const signal=init.signal?AbortSignal.any([init.signal,lifecycle.current.signal]):lifecycle.current.signal;
  signal.throwIfAborted();const value=await api<T>(path,{...init,signal});signal.throwIfAborted();return value;
 }
 function fail(error:unknown){if(lifecycle.current.signal.aborted||(error instanceof DOMException&&error.name==='AbortError'))return;setFailure(requestErrorMessage(error));setDetailOpen(true);if(isApiRequestError(error)&&error.code==='unauthenticated')callbacks.current.onError(error);}
 async function mutate(action:()=>Promise<void>){mutating.current=true;reads.cancel();reading.current=false;setBusy(true);setActivity('Updating import…');setFailure('');try{await action();}catch(error){fail(error);}finally{mutating.current=false;if(!lifecycle.current.signal.aborted){setBusy(false);setActivity('');setUploadProgress(undefined);}}}
 async function loadDetail(id:string){
  reading.current=true;
  await reads.run(async attempt=>{
   const value=await attempt.wait(request<Detail>(`/ingests/${id}`,{signal:attempt.signal}));
   const associated=value.import.association.item_id;
   const item=associated?await request<Item>(`/items/${associated}`,{signal:attempt.signal}):null;
   return {value,item};
  },{success:({value,item})=>{
   setDetail(current=>retainUnchanged(current,value));if(!targetEdited.current)setTarget(current=>retainUnchanged(current,item));
   if(value.import.phase==='published'&&item){const marker=JSON.stringify(item);if(notified.current.get(item.id)!==marker){notified.current.set(item.id,marker);callbacks.current.onPublished(item);}}
  },failure:fail,settled:()=>{reading.current=false;}});
 }
 async function open(id:string){setDetailOpen(true);setTarget(undefined);targetEdited.current=false;detailId.current=id;await loadDetail(id);}
 useEffect(()=>{
  const controller=new AbortController();lifecycle.current=controller;let timer:ReturnType<typeof setTimeout>|undefined;
  async function poll(){const id=detailId.current;if(id&&!reading.current&&!mutating.current)await loadDetail(id);if(!controller.signal.aborted)timer=setTimeout(poll,2000);}
  void poll();
  return()=>{controller.abort();reads.cancel();reading.current=false;if(timer)clearTimeout(timer);callbacks.current.setBusy(false);};
 },[collectionId]);
 useEffect(()=>{if(!uploadProgress)return;const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue='';};window.addEventListener('beforeunload',warn);return()=>window.removeEventListener('beforeunload',warn);},[!!uploadProgress]);
 useEffect(()=>{if(initialImportId)void open(initialImportId);},[initialImportId]);
 async function upload(value:Detail,selectedFiles:File[],verifyPrefix:boolean){
  const names=new Set(value.import.files.map(file=>file.name));
  await uploadPackage(value.import,selectedFiles.filter(file=>names.has(fileName(file))),verifyPrefix,lifecycle.current.signal,setUploadProgress,record=>{
   if(detailId.current===record.id)setDetail(current=>current?.summary.import_id===record.id?{...current,import:record}:current);
  });
  setUploadProgress(undefined);setActivity('Refreshing import status…');
 }
 async function resume(){if(!detail)return;await mutate(async()=>{const current=await request<Detail>(`/ingests/${detail.summary.import_id}`);if(current.import.phase!=='staging')throw new Error('This import no longer needs an upload. Refresh it.');await upload(current,resumeFiles,true);setResumeFiles([]);await open(current.summary.import_id);});}
 async function discard(){if(!detail||!confirm(`Discard ${detail.summary.package_name}? Its uploaded files will be deleted.`))return;await mutate(async()=>{await request(`/ingests/${detail.summary.import_id}/discard`,{method:'POST',body:JSON.stringify({expected_revision:detail.import.revision})});setNotice('Import discarded.');await open(detail.summary.import_id);});}
 async function cancel(){if(!detail)return;await mutate(async()=>{setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/cancel`,{method:'POST',body:JSON.stringify({expected_revision:detail.import.revision})}));});}
 async function retry(jobId?:string|null){if(!detail)return;await mutate(async()=>{setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/retry`,{method:'POST',body:JSON.stringify({job_id:jobId||null})}));});}
 async function cleanInbox(){if(!detail||!inboxConfirmed)return;const id=detail.import.id;await mutate(async()=>{
  const draftKey=`collector:inbox-cleanup:${getApiIdentity()?.user_id}:${id}`;
  const pending=await loadDraft<{key:string}>(draftKey)||{key:crypto.randomUUID()};
  await saveDraft(draftKey,pending);
  await request(`/ingests/${id}/inbox-cleanup`,{method:'POST',headers:{'Idempotency-Key':pending.key},body:JSON.stringify({source_is_idle:true})});
  await saveDraft(draftKey,undefined);setInboxConfirmed(false);await loadDetail(id);
 });}
 async function cancelInboxCleanup(){if(!detail?.inbox_cleanup)return;const id=detail.import.id;await mutate(async()=>{
  await request(`/jobs/${detail.inbox_cleanup!.job_id}/cancel`,{method:'POST'});await loadDetail(id);
 });}
 async function correct(){if(!detail||!associationChanged||detail.summary.capture_revision===null)return;await mutate(async()=>{
  const selected=target?await request<Item>(`/items/${target.id}`):null;
  if(selected&&selected.collection_id!==destinationCollection)throw new Error('The destination is outside this collection.');
  if(selected&&selected.revision!==target?.revision)throw new Error('The destination changed elsewhere. Search for it again.');
  const value=await request<Detail>(`/ingests/${detail.summary.import_id}/association`,{method:'PUT',body:JSON.stringify({expected_capture_revision:detail.summary.capture_revision,association:{item_id:selected?.id||null,specimen_id:null,component_id:null,carrier_id:null},expected_item_revision:selected?.revision||null})});setDetail(value);targetEdited.current=false;setTarget(selected);setNotice(selected?`Dump attached to ${selected.title}.`:'Import detached from the entry. Original files are kept.');if(selected)callbacks.current.onPublished(await request<Item>(`/items/${selected.id}`),detail.import.association.item_id||undefined);await open(detail.summary.import_id);
 });}
 return <section className="guided-ingest" data-uploading={!!uploadProgress} data-unsaved={associationChanged} aria-labelledby="guided-ingest-heading">
  <h3 id="guided-ingest-heading">Import progress</h3>
  {!detail&&!failure&&<p role="status">Loading import…</p>}
  {failure&&<p role="alert" className="notice error">{failure} Open the import below to resume.</p>}
  {notice&&<p role="status" className="notice">{notice}</p>}
  {uploadProgress?<UploadMeter value={uploadProgress}/>:activity&&<WorkProgress label={activity}/>}
  {detail&&<details className="ingest-detail" open={detailOpen} onToggle={event=>setDetailOpen(event.currentTarget.open)}><summary>Details: {detail.summary.package_name}</summary>
   <IngestStatus summary={detail.summary}/>
   {detail.import.phase==='published'&&!!detail.import.association.item_id&&<p className="notice">This dump is attached to {target?.id===detail.import.association.item_id?target.title:'its saved collection entry'}.</p>}
   {!uploadProgress&&detail.import.phase==='staging'&&<WorkProgress label={detail.import.source.kind==='browser'?'Upload progress':'Original files copied'} completed={detail.import.files.reduce((sum,file)=>sum+file.received_bytes,0)} total={detail.summary.total_bytes}/>}
   {detail.import.source.kind==='inbox'&&<section aria-label="Inbox source cleanup" className="notice inbox-cleanup">
    <strong>Inbox source files</strong>
    <p>Inbox files stay until you remove them. Removal is available once the import finishes. Other inbox imports pause while files are removed.</p>
    {detail.inbox_cleanup&&<>
     <p role="status">{detail.inbox_cleanup.completed?'Inbox cleanup complete':`Cleanup ${stateLabel(detail.inbox_cleanup.state)}`} · Confirmed removed {bytes(detail.inbox_cleanup.removed_bytes)} of {bytes(detail.inbox_cleanup.total_bytes)}</p>
     {detail.inbox_cleanup.error&&<p role="alert">{detail.inbox_cleanup.error}</p>}
     {detail.inbox_cleanup.recovery_directory&&(!detail.inbox_cleanup.completed||detail.inbox_cleanup.retained_files)&&<p>Recovery location: <code>{detail.inbox_cleanup.recovery_directory}</code> in the inbox. Files that could not be moved stay where they were. Retry to continue.</p>}
     {detail.inbox_cleanup.retained_files&&<p>Files that were not imported are in that folder. They were not deleted.</p>}
     {['queued','running','cancel_requested'].includes(detail.inbox_cleanup.state)&&!detail.inbox_cleanup.completed&&<button disabled={busy||detail.inbox_cleanup.state==='cancel_requested'} onClick={()=>void cancelInboxCleanup()}>Cancel inbox cleanup</button>}
    </>}
    {detail.summary.permissions.can_clean_inbox&&!detail.inbox_cleanup?.completed&&(!detail.inbox_cleanup||['failed','cancelled'].includes(detail.inbox_cleanup.state))&&<>
     <p>Only files that match the archive copies are removed. The inbox must allow deleting files.</p>
     <label><input type="checkbox" checked={inboxConfirmed} disabled={busy} onChange={event=>setInboxConfirmed(event.target.checked)}/> I'm done with these inbox files. Remove them; the archive copies stay.</label>
     <button disabled={busy||!inboxConfirmed} onClick={()=>void cleanInbox()}>{detail.inbox_cleanup?'Retry inbox cleanup':'Remove imported inbox files'}</button>
    </>}
   </section>}
   {detail.cleanup&&<p role="status">Temporary files cleared: {bytes(detail.cleanup.reclaimed_bytes)} · Remaining: {bytes(detail.cleanup.pending_bytes)}{detail.cleanup.cleanup_pending?' · cleanup pending':' · cleanup complete'}</p>}
   {detail.attempts.length>0&&<details><summary>Technical details: previous attempts</summary>{detail.attempts.map((attempt,index)=><p key={`${attempt.job_id}:${attempt.attempt}:${index}`}><strong>{stateLabel(attempt.step)} · attempt {attempt.attempt}</strong> — {stateLabel(attempt.state)}{attempt.diagnostic&&<> · {attempt.diagnostic}</>}</p>)}</details>}
   {detail.summary.capture_id&&<details><summary>Technical details</summary><p>Import ID: {detail.summary.import_id}</p><p>Capture ID: {detail.summary.capture_id}</p></details>}
   <div className="ingest-steps">{detail.steps.map(step=><div key={step.step}><strong>{step.step==='staging'&&detail.import.source.kind==='inbox'?'Copy files from inbox':stateLabel(step.step)}</strong><span>{stateLabel(step.state)}</span>{step.message&&<p>{step.message}</p>}{step.retryable&&<button disabled={busy} onClick={()=>void retry(step.job_id)}>Retry this step</button>}</div>)}</div>
   {detail.import.limitations.length>1&&<details><summary>File analysis details</summary><ul>{detail.import.limitations.map((message,index)=><li key={index}>{message}</li>)}</ul></details>}
   {!uploadProgress&&detail.summary.permissions.can_resume_upload&&<form aria-label="Resume guided package" onSubmit={event=>{event.preventDefault();void resume();}}><p>Select the same folder or files to continue the upload.</p><label>Original folder<input type="file" multiple {...{webkitdirectory:''}} disabled={busy} onChange={event=>setResumeFiles(Array.from(event.target.files||[]))}/></label><label>Original files<input aria-label="Resume original package files" type="file" multiple disabled={busy} onChange={event=>setResumeFiles(Array.from(event.target.files||[]))}/></label><button disabled={busy||!resumeFiles.length}>Resume upload</button></form>}
   {detail.summary.permissions.can_cancel&&<button disabled={busy} onClick={()=>void cancel()}>Cancel remaining work</button>}{detail.summary.permissions.can_discard&&<button disabled={busy} onClick={()=>void discard()}>Discard unfinished import</button>}
   {detail.summary.outcome==='failed'&&!detail.steps.some(step=>step.retryable)&&<button disabled={busy} onClick={()=>void retry(null)}>Retry optional processing</button>}
   {detail.summary.permissions.can_correct_association&&<details><summary>Correct destination</summary>{target===undefined?<p role="status">Loading saved destination…</p>:<><DestinationChooser collectionId={destinationCollection} selected={target} disabled={busy} onSelect={value=>{targetEdited.current=true;setTarget(value);}}/>{associationChanged?<><p className="muted">This moves only this import.</p><button disabled={busy} onClick={()=>void correct()}>Save destination</button></>:<p role="status">{detail.import.association.item_id?'Already attached to this entry.':'Not attached to an entry.'}</p>}</>}</details>}
  </details>}
 </section>;
}
