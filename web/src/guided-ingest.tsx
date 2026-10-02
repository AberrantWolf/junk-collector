import {uploadPackage,type UploadProgress} from './package-upload';
import {WorkProgress,UploadMeter,bytes} from './work-progress';
import React,{useEffect,useRef,useState,api,useOperationScope,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import DestinationChooser from './destination-chooser';
import InboxPackageChooser from './inbox-package-chooser';
type Discovery=components['schemas']['DiscoveredIngestPlan'];
type Plan=components['schemas']['IngestPlan'];
type Accepted=components['schemas']['AcceptedIngest'];
type Detail=components['schemas']['IngestDetail'];
type Item=components['schemas']['PresentedItem'];
type InboxSource=components['schemas']['InboxSourceView'];
type Props={initialImportId?:string;historyOnly?:boolean;initialItem?:Item;collectionId:string;busy:boolean;setBusy:(value:boolean)=>void;onPublished:(item:Item,replacedItemId?:string)=>void;onError:(error:unknown)=>void;onDraftChange?:(value:boolean)=>void};
function selectedName(files:File[]){return files.length===1?files[0].name:'Browser package';}
function fileName(file:File){return file.webkitRelativePath||file.name;}
function stateLabel(value:string){return value.replaceAll('_',' ');}

export default function GuidedIngest({initialImportId,historyOnly=false,initialItem,collectionId,busy,setBusy,onPublished,onError,onDraftChange}:Props){
 const [source,setSource]=useState<'browser'|'inbox'>('browser');
 const [folderMode,setFolderMode]=useState(true);const [unassigned,setUnassigned]=useState<string[]>([]);const [queueLabel,setQueueLabel]=useState('');
 const [files,setFiles]=useState<File[]>([]);const [resumeFiles,setResumeFiles]=useState<File[]>([]);
 const [packageName,setPackageName]=useState('Browser package');
 const [inboxes,setInboxes]=useState<InboxSource[]>([]);const [inboxLabel,setInboxLabel]=useState('');const [inboxPackages,setInboxPackages]=useState<string[]>([]);
 const [plan,setPlan]=useState<Plan>();const [automatic,setAutomatic]=useState(true);const [prepareMusic,setPrepareMusic]=useState(false);const [preserveOnly,setPreserveOnly]=useState(false);
 const [destination,setDestination]=useState<Item|null>(initialItem||null);
 useEffect(()=>{if(initialItem)setDestination(initialItem);},[initialItem?.id,initialItem?.revision]);
 const [detail,setDetail]=useState<Detail>();const [detailOpen,setDetailOpen]=useState(false);const [target,setTarget]=useState<Item|null>();
 const [uploadProgress,setUploadProgress]=useState<UploadProgress>();const [activity,setActivity]=useState('');
 const [notice,setNotice]=useState('');const [failure,setFailure]=useState('');
 const lifecycle=useRef(new AbortController());const reads=useOperationScope(collectionId);const reading=useRef(false);const mutating=useRef(false);
 const acceptance=useRef<string|undefined>(undefined);const detailId=useRef<string|undefined>(undefined);
 const targetEdited=useRef(false);
 const associationChanged=!!detail&&target!==undefined&&(target?.id??null)!==(detail.import.association.item_id??null);
 const callbacks=useRef({onPublished,onError,setBusy});callbacks.current={onPublished,onError,setBusy};
 const notified=useRef(new Map<string,string>());
 useEffect(()=>{onDraftChange?.(!!files.length||!!plan||!!inboxPackages.length&&source==='inbox');},[files,plan,inboxPackages,source,onDraftChange]);
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
   setDetail(value);if(!targetEdited.current)setTarget(item);
   if(value.import.phase==='published'&&item){const marker=JSON.stringify(item);if(notified.current.get(item.id)!==marker){notified.current.set(item.id,marker);callbacks.current.onPublished(item);}}
  },failure:fail,settled:()=>{reading.current=false;}});
 }
 async function open(id:string){setDetailOpen(true);setTarget(undefined);targetEdited.current=false;detailId.current=id;await loadDetail(id);}
 useEffect(()=>{
  lifecycle.current=new AbortController();let timer:ReturnType<typeof setTimeout>|undefined;
  async function poll(){const id=detailId.current;if(id&&!reading.current&&!mutating.current)await loadDetail(id);if(!lifecycle.current.signal.aborted)timer=setTimeout(poll,2000);}
  void poll();if(!historyOnly)void request<InboxSource[]>(`/collections/${collectionId}/inboxes?packages=false`).then(value=>{setInboxes(value);setInboxLabel(value[0]?.label||'');}).catch(fail);
  return()=>{lifecycle.current.abort();reads.cancel();reading.current=false;if(timer)clearTimeout(timer);callbacks.current.setBusy(false);};
 },[collectionId,historyOnly]);
 useEffect(()=>{if(!uploadProgress)return;const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue='';};window.addEventListener('beforeunload',warn);return()=>window.removeEventListener('beforeunload',warn);},[!!uploadProgress]);
 useEffect(()=>{if(initialImportId)void open(initialImportId);},[initialImportId]);
 async function discover(selected:File[]){setFiles(selected);setFolderMode(true);setPlan(undefined);setUnassigned([]);await mutate(async()=>{
  setActivity('Finding dump packages…');
  const result=await request<Discovery>('/ingests/discover',{method:'POST',body:JSON.stringify({collection_id:collectionId,files:selected.map(file=>({name:fileName(file),bytes:file.size,sha256:null}))})});
  setPlan(result.plan||undefined);setPreserveOnly(!!result.plan&&result.plan.packages.every(pkg=>!pkg.analysis_available));setPrepareMusic(current=>current&&!!result.plan&&result.plan.packages.every(pkg=>pkg.preparation_available));setUnassigned(result.unassigned_files);acceptance.current=crypto.randomUUID();
  if(!result.plan)setNotice('No supported dump packages were found. Nothing was uploaded.');else setNotice('');
 });}
 async function preview(){await mutate(async()=>{setActivity('Inspecting package…');
  const browser=source==='browser';const value=await request<Plan>('/ingests/plan',{method:'POST',body:JSON.stringify({collection_id:collectionId,packages:browser?[{source:{kind:source,label:null,package_name:packageName},files:files.map(file=>({name:fileName(file),bytes:file.size,sha256:null})),anchor_file_index:null}]:inboxPackages.map(name=>({source:{kind:'inbox',label:inboxLabel,package_name:name},files:[],anchor_file_index:null}))})});setPlan(value);setPreserveOnly(value.packages.every(pkg=>!pkg.analysis_available));setPrepareMusic(current=>current&&value.packages.every(pkg=>pkg.preparation_available));acceptance.current=crypto.randomUUID();
 });}
 async function chooseAnchor(position:number,fileIndex:number){if(!plan)return;await mutate(async()=>{
  const value=await request<Plan>('/ingests/plan',{method:'POST',body:JSON.stringify({collection_id:collectionId,packages:plan.packages.map(pkg=>({source:pkg.source,files:pkg.files,anchor_file_index:pkg.position===position?fileIndex:(pkg.anchors.find(anchor=>anchor.selected)?.file_index??null)}))})});setPlan(value);setPreserveOnly(value.packages.every(pkg=>!pkg.analysis_available));setPrepareMusic(current=>current&&value.packages.every(pkg=>pkg.preparation_available));acceptance.current=crypto.randomUUID();
 });}
 async function upload(value:Detail,selectedFiles:File[],verifyPrefix:boolean){
  const names=new Set(value.import.files.map(file=>file.name));
  await uploadPackage(value.import,selectedFiles.filter(file=>names.has(fileName(file))),verifyPrefix,lifecycle.current.signal,setUploadProgress,record=>{
   if(detailId.current===record.id)setDetail(current=>current?.summary.import_id===record.id?{...current,import:record}:current);
  });
  setUploadProgress(undefined);setActivity('Refreshing import status…');
 }
 async function start(){if(!plan)return;const frozen=plan;const selected=[...files];await mutate(async()=>{
  setDetailOpen(false);setActivity('Starting import…');acceptance.current??=crypto.randomUUID();
  if(destination&&frozen.packages.length!==1)throw new Error('Select one package when attaching to an existing entry.');
  const currentDestination=destination?await request<Item>(`/items/${destination.id}`):null;
  if(currentDestination&&currentDestination.collection_id!==collectionId)throw new Error('The destination is outside this collection.');
  if(currentDestination&&currentDestination.revision!==destination?.revision)throw new Error('The destination changed. Reopen it and review the current revision before importing.');
  const accepted=await request<Accepted>('/ingests',{method:'POST',headers:{'Idempotency-Key':acceptance.current},body:JSON.stringify({plan:frozen,analysis:preserveOnly?'preserve_only':'automatic',enrichment:automatic&&!preserveOnly?'automatic':'none',prepare_music:prepareMusic&&!preserveOnly,destination:currentDestination?{item_id:currentDestination.id,expected_revision:currentDestination.revision}:null})});
  if(!accepted.imports.length)throw new Error(accepted.rejected[0]?.reason||'No package was accepted.');
  setPlan(undefined);
  for(let index=0;index<accepted.imports.length;index++){
   const summary=accepted.imports[index];setQueueLabel(`Package ${index+1} of ${accepted.imports.length}: ${summary.package_name}`);
   reads.cancel();reading.current=false;targetEdited.current=false;detailId.current=summary.import_id;const current=await request<Detail>(`/ingests/${summary.import_id}`);setDetail(current);
   if(current.import.source.kind==='browser'&&current.import.phase==='staging')await upload(current,selected,false);
  }
  setQueueLabel('');setDetailOpen(true);setFiles([]);setNotice(accepted.rejected.length?`Uploads finished. ${accepted.rejected.length} packages were not accepted: ${accepted.rejected.map(value=>value.reason).join('; ')}`:frozen.packages.every(pkg=>pkg.source.kind==='browser')?'Upload complete. Your entries are saved. You can close this window; identification, metadata and optional audio preparation continue on the server.':'Import queued. Your entries are saved. You can close this window; the server will copy the source packages and continue processing.');

 });}
 async function resume(){if(!detail)return;await mutate(async()=>{const current=await request<Detail>(`/ingests/${detail.summary.import_id}`);if(current.import.phase!=='staging')throw new Error('This import no longer needs an upload. Refresh its detail.');await upload(current,resumeFiles,true);setResumeFiles([]);await open(current.summary.import_id);});}
 async function discard(){if(!detail||!confirm(`Discard ${detail.summary.package_name} and reclaim its unpublished staging bytes?`))return;await mutate(async()=>{await request(`/ingests/${detail.summary.import_id}/discard`,{method:'POST',body:JSON.stringify({expected_revision:detail.import.revision})});setNotice('Discard accepted. Cleanup progress is shown below.');await open(detail.summary.import_id);});}
 async function cancel(){if(!detail)return;await mutate(async()=>{setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/cancel`,{method:'POST',body:JSON.stringify({expected_revision:detail.import.revision})}));});}
 async function retry(jobId?:string|null){if(!detail)return;await mutate(async()=>{setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/retry`,{method:'POST',body:JSON.stringify({job_id:jobId||null})}));});}
 async function correct(){if(!detail||!associationChanged||detail.summary.capture_revision===null)return;await mutate(async()=>{
  const selected=target?await request<Item>(`/items/${target.id}`):null;
  if(selected&&selected.collection_id!==collectionId)throw new Error('The destination is outside this collection.');
  if(selected&&selected.revision!==target?.revision)throw new Error('The destination changed. Search for it again and review its current revision before attaching.');
  const value=await request<Detail>(`/ingests/${detail.summary.import_id}/association`,{method:'PUT',body:JSON.stringify({expected_capture_revision:detail.summary.capture_revision,association:{item_id:selected?.id||null,specimen_id:null,component_id:null,carrier_id:null},expected_item_revision:selected?.revision||null})});setDetail(value);targetEdited.current=false;setTarget(selected);setNotice(selected?`Dump attached to ${selected.title}. Other discs are unchanged. Unused automatic placeholders are removed; entries with independent inventory or manual information are kept.`:'Dump association removed. Original files are retained.');if(selected)callbacks.current.onPublished(await request<Item>(`/items/${selected.id}`),detail.import.association.item_id||undefined);await open(detail.summary.import_id);
 });}
 return <section className="guided-ingest" data-uploading={!!uploadProgress} aria-labelledby="guided-ingest-heading">
  <h3 id="guided-ingest-heading">{historyOnly?'Import progress':'Import original packages'}</h3>{!historyOnly&&<>
  <p className="muted">Choose a folder of dumps. We find packages in its subfolders, create entries, preserve the originals, and gather available metadata and artwork.</p>
  </>}
  {failure&&<p role="alert" className="notice error">{failure} Saved entries and received bytes are kept. Open the import below to resume.</p>}
  {notice&&<p role="status" className="notice">{notice}</p>}
  {queueLabel&&<p><strong>{queueLabel}</strong></p>}
  {uploadProgress?<UploadMeter value={uploadProgress}/>:activity&&<WorkProgress label={activity}/>}
  {!historyOnly&&<><form onSubmit={event=>{event.preventDefault();void (plan?start():preview());}}>
   <DestinationChooser collectionId={collectionId} selected={destination} disabled={busy||!!initialItem} emptyMode="create" onSelect={value=>{setDestination(value);acceptance.current=undefined;}}/>{initialItem&&<p className="muted">This import will attach to the open entry. Use Add / import to choose another destination.</p>}
   <fieldset className="source-choice"><legend>Import from</legend><label><input type="radio" name="guided-source" disabled={busy} checked={source==='browser'} onChange={()=>{setSource('browser');setPlan(undefined);}}/> This browser</label><label><input type="radio" name="guided-source" disabled={busy} checked={source==='inbox'} onChange={()=>{setSource('inbox');setPlan(undefined);}}/> Configured inbox</label></fieldset>
   {source==='browser'?<>
    <label>Dump folder<input aria-label="Dump folder" type="file" multiple {...{webkitdirectory:''}} disabled={busy} onChange={event=>{const selected=Array.from(event.target.files||[]);if(selected.length)void discover(selected);}}/></label>
    <details><summary>Choose individual files instead</summary><label>Package name<input value={packageName} disabled={busy} onChange={event=>{setPackageName(event.target.value);setPlan(undefined);}}/></label><label>Package files<input aria-label="Package files" type="file" multiple disabled={busy} onChange={event=>{const selected=Array.from(event.target.files||[]);setFolderMode(false);setUnassigned([]);setFiles(selected);setPackageName(selectedName(selected));setPlan(undefined);}}/></label></details>
    {files.length>0&&<p className="muted">{files.length} files selected · {bytes(files.reduce((sum,file)=>sum+file.size,0))}</p>}
   </>:inboxes.length?<div><label>Inbox<select value={inboxLabel} disabled={busy} onChange={event=>{setInboxLabel(event.target.value);setPlan(undefined);}}>{inboxes.map(value=><option key={value.label}>{value.label}</option>)}</select></label><InboxPackageChooser collectionId={collectionId} label={inboxLabel} selected={inboxPackages} onSelect={value=>{setInboxPackages(value);setPlan(undefined);}} disabled={busy} multiple/></div>:<p className="notice">No configured inbox packages are available.</p>}
   {!!unassigned.length&&<details className="notice"><summary>{unassigned.length} {unassigned.length===1?'file is':'files are'} outside the detected packages and will not be uploaded</summary><ul>{unassigned.map(name=><li key={name}>{name}</li>)}</ul></details>}
   {plan&&<>
    <p><strong>{plan.packages.length} dump packages found.</strong> {destination?'The package will be attached to the selected entry; its copy records stay as they are.':'A new entry and one owned physical copy will be recorded for each package.'}</p>
    {plan.packages.some(pkg=>pkg.blockers.length>0)&&<div className="notice" role="note"><strong>Review before preserving</strong>{plan.packages.flatMap(pkg=>pkg.blockers.map(blocker=><p key={`${pkg.position}:${blocker.kind}:${blocker.message}`}>{pkg.default_title}: {blocker.message} {blocker.blocks_analysis?'Originals can still be preserved without analysis.':''}</p>))}</div>}
    {destination&&plan.packages.length!==1&&<p className="notice error" role="alert">Choose one package when attaching to an existing entry. Clear the destination to create separate entries for all packages.</p>}
    <label className="inline-check"><input type="checkbox" checked={preserveOnly} disabled={busy} onChange={event=>{setPreserveOnly(event.target.checked);if(event.target.checked)setPrepareMusic(false);}}/> Preserve originals without analysis or automatic lookup</label>
    <details className="ingest-preview"><summary>Package details and import options</summary>{plan.packages.map(pkg=><div key={pkg.position}><strong>{pkg.default_title}</strong><p>{pkg.files.length} files · destination: {destination?.title||'new collection entry'}</p>{pkg.anchors.length>1&&<fieldset><legend>Analysis anchor (optional)</legend>{pkg.anchors.map(anchor=><label key={anchor.file_index}><input type="radio" name={`anchor-${pkg.position}`} checked={anchor.selected} disabled={busy} onChange={()=>void chooseAnchor(pkg.position,anchor.file_index)}/>{anchor.label}: {pkg.files[anchor.file_index]?.name}</label>)}</fieldset>}<p className="muted">Metadata: {pkg.metadata_available?'available':'unavailable or not configured'} · Artwork: {pkg.artwork_available?'available':'unavailable or not configured'} · FLAC: {pkg.preparation_available?'available':pkg.preparation_reason||'unavailable'}</p>{pkg.blockers.map(blocker=><p className="notice" key={`${blocker.kind}:${blocker.message}`}>{blocker.message}</p>)}</div>)}<label><input type="checkbox" checked={automatic&&!preserveOnly} disabled={busy||preserveOnly} onChange={event=>setAutomatic(event.target.checked)}/> Automatically gather available metadata and artwork</label></details>
    <label><input type="checkbox" disabled={busy||preserveOnly||!plan.packages.every(pkg=>pkg.preparation_available)} checked={prepareMusic} onChange={event=>setPrepareMusic(event.target.checked)}/> Prepare supported music as FLAC after preservation</label>
   </>}
   {(plan||source==='inbox'||!folderMode)&&<button className="primary" disabled={busy||!!(destination&&plan&&plan.packages.length!==1)||(source==='browser'?!files.length:!inboxPackages.length)}>{plan?`Preserve ${plan.packages.length} ${plan.packages.length===1?'package':'packages'}`:'Preview import'}</button>}
  </form>
  <p className="muted">Keep this window open while uploading. Once uploads finish, you can close it: saved entries and server processing are independent of this window. No separate inventory save is needed.</p>
  <button disabled={busy} onClick={()=>window.dispatchEvent(new Event('collector-open-work'))}>Open work &amp; history</button>
  </>}
  {detail&&<details className="ingest-detail" open={detailOpen} onToggle={event=>setDetailOpen(event.currentTarget.open)}><summary>Details: {detail.summary.package_name}</summary>
   {detail.import.phase==='published'&&!!detail.import.association.item_id&&<p className="notice">This dump is attached to {target?.id===detail.import.association.item_id?target.title:'its saved collection entry'}. Other discs attached to that entry are retained.</p>}
   {!uploadProgress&&detail.import.phase==='staging'&&<WorkProgress label="Package staging progress" completed={detail.import.files.reduce((sum,file)=>sum+file.received_bytes,0)} total={detail.summary.total_bytes} detail="Received bytes and the entry are saved on the server."/>}
   {detail.cleanup&&<p role="status">Reclaimed {detail.cleanup.reclaimed_bytes.toLocaleString()} bytes · {detail.cleanup.pending_bytes.toLocaleString()} pending{detail.cleanup.cleanup_pending?' · cleanup pending':' · cleanup complete'}</p>}
   {detail.attempts.length>0&&<details><summary>Attempt history</summary>{detail.attempts.map((attempt,index)=><p key={`${attempt.job_id}:${attempt.attempt}:${index}`}><strong>{stateLabel(attempt.step)} · attempt {attempt.attempt}</strong> — {stateLabel(attempt.state)}{attempt.diagnostic&&<> · {attempt.diagnostic}</>}</p>)}</details>}
   {detail.summary.capture_id&&<p>Capture {detail.summary.capture_id}</p>}
   <div className="ingest-steps">{detail.steps.map(step=><div key={step.step}><strong>{stateLabel(step.step)}</strong><span>{stateLabel(step.state)}</span>{step.message&&<p>{step.message}</p>}{step.state==='active'&&<WorkProgress label={`${stateLabel(step.step)} in progress`} detail="The server is working; this view refreshes automatically."/>}{step.retryable&&<button disabled={busy} onClick={()=>void retry(step.job_id)}>Retry this step</button>}</div>)}</div>
   {!uploadProgress&&detail.summary.permissions.can_resume_upload&&<form aria-label="Resume guided package" onSubmit={event=>{event.preventDefault();void resume();}}><p>Select the original folder or files. Received bytes will be checked before the upload continues.</p><label>Original folder<input type="file" multiple {...{webkitdirectory:''}} disabled={busy} onChange={event=>setResumeFiles(Array.from(event.target.files||[]))}/></label><label>Original package files<input aria-label="Resume original package files" type="file" multiple disabled={busy} onChange={event=>setResumeFiles(Array.from(event.target.files||[]))}/></label><button disabled={busy||!resumeFiles.length}>Resume upload</button></form>}
   {detail.summary.permissions.can_cancel&&<button disabled={busy} onClick={()=>void cancel()}>Cancel remaining work</button>}{detail.summary.permissions.can_discard&&<button disabled={busy} onClick={()=>void discard()}>Discard unpublished import</button>}
   {detail.summary.outcome==='failed'&&!detail.steps.some(step=>step.retryable)&&<button disabled={busy} onClick={()=>void retry(null)}>Retry optional processing</button>}
   {detail.summary.permissions.can_correct_association&&<details><summary>Correct destination</summary>{target===undefined?<p role="status">Loading saved destination…</p>:<><DestinationChooser collectionId={collectionId} selected={target} disabled={busy} onSelect={value=>{targetEdited.current=true;setTarget(value);}}/>{associationChanged?<><p role="status">Destination changed. Save to apply this change.</p><button disabled={busy} onClick={()=>void correct()}>Save association</button><p className="muted">This moves only this dump's association. Other discs and original files are retained.</p></>:<p role="status">{detail.import.association.item_id?'Already attached to this entry. No changes to save.':'This dump is already unattached. No changes to save.'}</p>}</>}</details>}
  </details>}
 </section>;
}
