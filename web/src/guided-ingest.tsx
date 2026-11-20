import {ImportPreparation,importPreparation,type ImportPreparationChoice} from './audio-profiles';
import {loadDraft,saveDraft} from './intake-drafts';
import PurchaseFields from './purchase-fields';
import IngestStatus from './ingest-status';
import InfoHelp from './info-help';
import {presentationLabel} from './presentation-labels';
import {uploadPackage,type UploadProgress} from './package-upload';
import {WorkProgress,UploadMeter,bytes} from './work-progress';
import React,{useEffect,useRef,useState,api,getApiIdentity,useOperationScope,isApiRequestError,requestErrorMessage,retainUnchanged} from '@collector/runtime';
import type {components} from './generated/api';
import DestinationChooser from './destination-chooser';
import InboxPackageChooser from './inbox-package-chooser';
type Discovery=components['schemas']['DiscoveredIngestPlan'];
type Plan=components['schemas']['IngestPlan'];
type Accepted=components['schemas']['AcceptedIngest'];
type Detail=components['schemas']['IngestDetail'];
type Item=components['schemas']['PresentedItem'];
type InboxSource=components['schemas']['InboxSourceView'];
type PendingPurchase={key:string;command:components['schemas']['StartIngestCommand']};
type Props={initialImportId?:string;historyOnly?:boolean;initialItem?:Item;collectionId:string;busy:boolean;setBusy:(value:boolean)=>void;onPublished:(item:Item,replacedItemId?:string)=>void;onError:(error:unknown)=>void;onDraftChange?:(value:boolean)=>void};
function selectedName(files:File[]){const cues=files.filter(file=>file.name.toLowerCase().endsWith('.cue'));return files.length===1?files[0].name:cues.length===1?cues[0].name:'Browser package';}
function fileName(file:File){return file.webkitRelativePath||file.name;}
function stateLabel(value:string){return presentationLabel(value);}

export default function GuidedIngest({initialImportId,historyOnly=false,initialItem,collectionId,busy,setBusy,onPublished,onError,onDraftChange}:Props){
 const [source,setSource]=useState<'browser'|'inbox'>('browser');
 const [folderMode,setFolderMode]=useState(true);const [unassigned,setUnassigned]=useState<string[]>([]);const [queueLabel,setQueueLabel]=useState('');
 const [files,setFiles]=useState<File[]>([]);const [resumeFiles,setResumeFiles]=useState<File[]>([]);
 const [packageName,setPackageName]=useState('Browser package');
 const [inboxes,setInboxes]=useState<InboxSource[]>([]);const [inboxLabel,setInboxLabel]=useState('');const [inboxPackages,setInboxPackages]=useState<string[]>([]);
 const [plan,setPlan]=useState<Plan>();const [automatic,setAutomatic]=useState(true);const [audioChoice,setAudioChoice]=useState<ImportPreparationChoice>('default');const [preserveOnly,setPreserveOnly]=useState(false);
 const [pendingPurchase,setPendingPurchase]=useState<PendingPurchase>();
 const pendingKey=`collector:guided-purchase:${getApiIdentity()?.user_id}:${collectionId}:${initialItem?.id||'new'}`;
 const [purchase,setPurchase]=useState<components['schemas']['DigitalPurchase']>();
 useEffect(()=>{if(historyOnly)return;const controller=new AbortController();void loadDraft<PendingPurchase>(pendingKey).then(value=>{if(!controller.signal.aborted)setPendingPurchase(value);}).catch(onError);return()=>controller.abort();},[pendingKey,historyOnly]);
 const [destination,setDestination]=useState<Item|null>(initialItem||null);
 useEffect(()=>{if(initialItem)setDestination(initialItem);},[initialItem?.id,initialItem?.revision]);
 const [detail,setDetail]=useState<Detail>();const [detailOpen,setDetailOpen]=useState(false);const [target,setTarget]=useState<Item|null>();
 const [uploadProgress,setUploadProgress]=useState<UploadProgress>();const [activity,setActivity]=useState('');
 const [inboxConfirmed,setInboxConfirmed]=useState(false);
 useEffect(()=>setInboxConfirmed(false),[detail?.import.id]);
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
   setDetail(current=>retainUnchanged(current,value));if(!targetEdited.current)setTarget(current=>retainUnchanged(current,item));
   if(value.import.phase==='published'&&item){const marker=JSON.stringify(item);if(notified.current.get(item.id)!==marker){notified.current.set(item.id,marker);callbacks.current.onPublished(item);}}
  },failure:fail,settled:()=>{reading.current=false;}});
 }
 async function open(id:string){setDetailOpen(true);setTarget(undefined);targetEdited.current=false;detailId.current=id;await loadDetail(id);}
 useEffect(()=>{
  const controller=new AbortController();lifecycle.current=controller;let timer:ReturnType<typeof setTimeout>|undefined;
  async function poll(){const id=detailId.current;if(id&&!reading.current&&!mutating.current)await loadDetail(id);if(!controller.signal.aborted)timer=setTimeout(poll,2000);}
  void poll();if(!historyOnly)void request<InboxSource[]>(`/collections/${collectionId}/inboxes?packages=false`).then(value=>{setInboxes(value);setInboxLabel(value[0]?.label||'');}).catch(fail);
  return()=>{controller.abort();reads.cancel();reading.current=false;if(timer)clearTimeout(timer);callbacks.current.setBusy(false);};
 },[collectionId,historyOnly]);
 useEffect(()=>{if(!uploadProgress)return;const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue='';};window.addEventListener('beforeunload',warn);return()=>window.removeEventListener('beforeunload',warn);},[!!uploadProgress]);
 useEffect(()=>{if(initialImportId)void open(initialImportId);},[initialImportId]);
 async function discover(selected:File[]){setFiles(selected);setFolderMode(true);setPlan(undefined);setUnassigned([]);await mutate(async()=>{
  setActivity('Finding dump packages…');
  const result=await request<Discovery>('/ingests/discover',{method:'POST',body:JSON.stringify({collection_id:collectionId,files:selected.map(file=>({name:fileName(file),bytes:file.size,sha256:null}))})});
  setPlan(result.plan||undefined);setPreserveOnly(!!result.plan&&result.plan.packages.every(pkg=>!pkg.analysis_available));setAudioChoice(current=>current==='flac'&&!result.plan?.packages.every(pkg=>pkg.preparation_available)?'default':current);setUnassigned(result.unassigned_files);acceptance.current=crypto.randomUUID();
  if(!result.plan)setNotice('No supported dumps found. Nothing was uploaded.');else setNotice('');
 });}
 async function preview(){await mutate(async()=>{setActivity('Inspecting package…');
  const browser=source==='browser';const value=await request<Plan>('/ingests/plan',{method:'POST',body:JSON.stringify({collection_id:collectionId,packages:browser?[{source:{kind:source,label:null,package_name:packageName},files:files.map(file=>({name:fileName(file),bytes:file.size,sha256:null})),anchor_file_index:null}]:inboxPackages.map(name=>({source:{kind:'inbox',label:inboxLabel,package_name:name},files:[],anchor_file_index:null}))})});setPlan(value);setPreserveOnly(value.packages.every(pkg=>!pkg.analysis_available));setAudioChoice(current=>current==='flac'&&!value.packages.every(pkg=>pkg.preparation_available)?'default':current);acceptance.current=crypto.randomUUID();
 });}
 async function chooseAnchor(position:number,fileIndex:number){if(!plan)return;await mutate(async()=>{
  const value=await request<Plan>('/ingests/plan',{method:'POST',body:JSON.stringify({collection_id:collectionId,packages:plan.packages.map(pkg=>({source:pkg.source,files:pkg.files,anchor_file_index:pkg.position===position?fileIndex:(pkg.anchors.find(anchor=>anchor.selected)?.file_index??null)}))})});setPlan(value);setPreserveOnly(value.packages.every(pkg=>!pkg.analysis_available));setAudioChoice(current=>current==='flac'&&!value.packages.every(pkg=>pkg.preparation_available)?'default':current);acceptance.current=crypto.randomUUID();
 });}
 async function changeVisibility(position:number,index:number,visibility:'shared'|'curators'){
  if(!plan)return;await mutate(async()=>{const value=await request<Plan>('/ingests/plan',{method:'POST',body:JSON.stringify({collection_id:collectionId,packages:plan.packages.map(pkg=>({purchase:pkg.purchase,source:pkg.source,files:pkg.files.map((f,i)=>pkg.position===position&&i===index?{...f,visibility}:f),anchor_file_index:pkg.anchors.find(a=>a.selected)?.file_index??null}))})});setPlan(value);acceptance.current=crypto.randomUUID();});
 }
 async function upload(value:Detail,selectedFiles:File[],verifyPrefix:boolean){
  const names=new Set(value.import.files.map(file=>file.name));
  await uploadPackage(value.import,selectedFiles.filter(file=>names.has(fileName(file))),verifyPrefix,lifecycle.current.signal,setUploadProgress,record=>{
   if(detailId.current===record.id)setDetail(current=>current?.summary.import_id===record.id?{...current,import:record}:current);
  });
  setUploadProgress(undefined);setActivity('Refreshing import status…');
 }
 async function start(){if(!plan&&!pendingPurchase)return;const frozen=pendingPurchase?.command.plan||plan!;const selected=[...files];await mutate(async()=>{
  setDetailOpen(false);setActivity('Starting import…');acceptance.current??=crypto.randomUUID();
  if(!pendingPurchase&&destination&&frozen.packages.length!==1)throw new Error('Select one package when attaching to an existing entry.');
  const currentDestination=!pendingPurchase&&destination?await request<Item>(`/items/${destination.id}`):null;
  if(currentDestination&&currentDestination.collection_id!==collectionId)throw new Error('The destination is outside this collection.');
  if(currentDestination&&currentDestination.revision!==destination?.revision)throw new Error('The destination changed elsewhere. Reopen it and try again.');
  const submitted=pendingPurchase||{key:acceptance.current,command:{purchase:purchase||null,plan:frozen,analysis:preserveOnly?'preserve_only' as const:'automatic' as const,enrichment:automatic&&!preserveOnly?'automatic' as const:'none' as const,prepare_music:false,audio_preparation:preserveOnly||!frozen.packages.every(pkg=>pkg.music_actions_available)?undefined:importPreparation(audioChoice),destination:currentDestination?{item_id:currentDestination.id,expected_revision:currentDestination.revision}:null}};
  await saveDraft(pendingKey,submitted);setPendingPurchase(submitted);
  const accepted=await request<Accepted>('/ingests',{method:'POST',headers:{'Idempotency-Key':submitted.key},body:JSON.stringify(submitted.command)});
  await saveDraft(pendingKey,undefined);setPendingPurchase(undefined);
  if(!accepted.imports.length)throw new Error(accepted.rejected.map(value=>`Package ${value.position+1}: ${value.reason}`).join('; ')||'No package was accepted.');
  setPlan(undefined);
  for(let index=0;index<accepted.imports.length;index++){
   const summary=accepted.imports[index];setQueueLabel(`Package ${index+1} of ${accepted.imports.length}: ${summary.package_name}`);
   reads.cancel();reading.current=false;targetEdited.current=false;detailId.current=summary.import_id;const current=await request<Detail>(`/ingests/${summary.import_id}`);setDetail(current);
   if(current.import.source.kind==='browser'&&current.import.phase==='staging'&&selected.length)await upload(current,selected,false);
  }
  setQueueLabel('');setDetailOpen(true);setFiles([]);setNotice(accepted.rejected.length?`Uploads finished. ${accepted.rejected.length} packages were not accepted: ${accepted.rejected.map(value=>value.reason).join('; ')}`:frozen.packages.every(pkg=>pkg.source.kind==='browser')?'Upload complete. You can close this window; processing continues on the server.':'Import queued. You can close this window; the server copies the files and continues.');

 });}
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
  if(selected&&selected.collection_id!==collectionId)throw new Error('The destination is outside this collection.');
  if(selected&&selected.revision!==target?.revision)throw new Error('The destination changed elsewhere. Search for it again.');
  const value=await request<Detail>(`/ingests/${detail.summary.import_id}/association`,{method:'PUT',body:JSON.stringify({expected_capture_revision:detail.summary.capture_revision,association:{item_id:selected?.id||null,specimen_id:null,component_id:null,carrier_id:null},expected_item_revision:selected?.revision||null})});setDetail(value);targetEdited.current=false;setTarget(selected);setNotice(selected?`Dump attached to ${selected.title}.`:'Import detached from the entry. Original files are kept.');if(selected)callbacks.current.onPublished(await request<Item>(`/items/${selected.id}`),detail.import.association.item_id||undefined);await open(detail.summary.import_id);
 });}
 return <section className="guided-ingest" data-uploading={!!uploadProgress} data-unsaved={associationChanged} aria-labelledby="guided-ingest-heading">
  <h3 id="guided-ingest-heading">{historyOnly?'Import progress':'Import original files'}</h3>{!historyOnly&&<p className="muted">Choose a folder of dumps. Each subfolder becomes an entry.</p>}
  {initialImportId&&!detail&&!failure&&<p role="status">Loading import…</p>}
  {failure&&<p role="alert" className="notice error">{failure} Open the import below to resume.</p>}
  {notice&&<p role="status" className="notice">{notice}</p>}
  {queueLabel&&<p><strong>{queueLabel}</strong></p>}
  {uploadProgress?<UploadMeter value={uploadProgress}/>:activity&&<WorkProgress label={activity}/>}
  {!historyOnly&&<>{pendingPurchase&&<p className="notice">A submitted import is waiting for confirmation. <button disabled={busy} onClick={()=>void start()}>Resume submitted import</button></p>}<form onSubmit={event=>{event.preventDefault();void (plan?start():preview());}}>
   <DestinationChooser collectionId={collectionId} selected={destination} disabled={busy||!!initialItem} emptyMode="create" onSelect={value=>{setDestination(value);acceptance.current=undefined;}}/>{initialItem&&<p className="muted">Files will be attached to this entry.</p>}
   <fieldset className="source-choice"><legend>Import from</legend><label><input type="radio" name="guided-source" disabled={busy} checked={source==='browser'} onChange={()=>{setSource('browser');setPlan(undefined);}}/> This browser</label><label><input type="radio" name="guided-source" disabled={busy} checked={source==='inbox'} onChange={()=>{setSource('inbox');setPlan(undefined);}}/> Server inbox</label></fieldset>
   {source==='browser'?<>
    <label>Dump folder<input aria-label="Dump folder" type="file" multiple {...{webkitdirectory:''}} disabled={busy} onChange={event=>{const selected=Array.from(event.target.files||[]);if(selected.length)void discover(selected);}}/></label>
    <details><summary>Choose individual files instead</summary><label>Import name<input value={packageName} disabled={busy} onChange={event=>{setPackageName(event.target.value);setPlan(undefined);}}/></label><label>Files<input aria-label="Files" type="file" multiple disabled={busy} onChange={event=>{const selected=Array.from(event.target.files||[]);setFolderMode(false);setUnassigned([]);setFiles(selected);setPackageName(selectedName(selected));setPlan(undefined);}}/></label></details>
    {files.length>0&&<p className="muted">{files.length} files selected · {bytes(files.reduce((sum,file)=>sum+file.size,0))}</p>}
   </>:inboxes.length?<div><label>Inbox<select value={inboxLabel} disabled={busy} onChange={event=>{setInboxLabel(event.target.value);setPlan(undefined);}}>{inboxes.map(value=><option key={value.label}>{value.label}</option>)}</select></label><InboxPackageChooser collectionId={collectionId} label={inboxLabel} selected={inboxPackages} onSelect={value=>{setInboxPackages(value);setPlan(undefined);}} disabled={busy} multiple/></div>:<p className="notice">No inbox packages found.</p>}
   {!!unassigned.length&&<details className="notice"><summary>{unassigned.length} {unassigned.length===1?'file is':'files are'} outside the detected packages and will not be uploaded</summary><ul>{unassigned.map(name=><li key={name}>{name}</li>)}</ul></details>}
   {plan&&<>
    <p><strong>{plan.packages.length} dump packages found.</strong> {destination?'It will be attached to the selected entry.':plan.packages.every(pkg=>pkg.files.some(f=>/\.(flac|mp3|wav)$/i.test(f.name))&&!pkg.anchors.length)?'A digital entry will be created, with no physical copy.':'Each package becomes a new entry with one physical copy.'}</p>
    {plan.packages.every(pkg=>pkg.files.some(f=>/\.(flac|mp3|wav)$/i.test(f.name))&&!pkg.anchors.length)&&<PurchaseFields value={purchase} disabled={busy} onChange={value=>{setPurchase(value);acceptance.current=undefined;}}/>}
    {plan.packages.some(pkg=>pkg.blockers.length>0)&&<div className="notice" role="note"><strong>Review before saving</strong>{plan.packages.flatMap(pkg=>pkg.blockers.map(blocker=><p key={`${pkg.position}:${blocker.kind}:${blocker.message}`}>{pkg.default_title}: {blocker.message} {blocker.blocks_analysis?'You can still save the originals without analysis.':''}</p>))}</div>}
    {destination&&plan.packages.length!==1&&<p className="notice error" role="alert">Only one package can be attached to an existing entry. Clear the destination to create an entry for each.</p>}
    <label className="inline-check"><input type="checkbox" checked={preserveOnly} disabled={busy} onChange={event=>{setPreserveOnly(event.target.checked);}}/> Save originals only</label>
    <details className="ingest-preview"><summary>Package details and import options</summary>{plan.packages.map(pkg=><div key={pkg.position}><strong>{pkg.default_title}</strong><p>{pkg.files.length} files · destination: {destination?.title||'new collection entry'}</p><details><summary>File sharing</summary>{pkg.files.map((file,index)=><label key={file.name}><input type="checkbox" checked={file.visibility==='curators'} disabled={busy} onChange={e=>void changeVisibility(pkg.position,index,e.target.checked?'curators':'shared')}/> Curators only: {file.name}</label>)}</details>{pkg.anchors.length>1&&<fieldset><legend>Main file for analysis (optional) <InfoHelp label="main file">For example, the CUE sheet.</InfoHelp></legend>{pkg.anchors.map(anchor=><label key={anchor.file_index}><input type="radio" name={`anchor-${pkg.position}`} checked={anchor.selected} disabled={busy} onChange={()=>void chooseAnchor(pkg.position,anchor.file_index)}/>{anchor.label}: {pkg.files[anchor.file_index]?.name}</label>)}</fieldset>}<p className="muted">Metadata: {pkg.metadata_available?'available':'unavailable'} · Artwork: {pkg.artwork_available?'available':'unavailable'}{pkg.music_actions_available&&<> · FLAC: {pkg.preparation_available?'available':pkg.preparation_reason||'unavailable'}</>}</p>{pkg.blockers.map(blocker=><p className="notice" key={`${blocker.kind}:${blocker.message}`}>{blocker.message}</p>)}</div>)}<label><input type="checkbox" checked={automatic&&!preserveOnly} disabled={busy||preserveOnly} onChange={event=>setAutomatic(event.target.checked)}/> Find metadata and artwork automatically</label></details>
    {plan.packages.every(pkg=>pkg.music_actions_available)&&!preserveOnly&&<ImportPreparation value={audioChoice} onChange={setAudioChoice} settings={plan.opus_import??null} packages={plan.packages} canFlac={plan.packages.every(pkg=>pkg.preparation_available)} disabled={busy||!!pendingPurchase}/>}
   </>}
   {(plan||source==='inbox'||!folderMode)&&<button className="primary" disabled={busy||!!(destination&&plan&&plan.packages.length!==1)||(source==='browser'?!files.length:!inboxPackages.length)}>{plan?`Save ${plan.packages.length} ${plan.packages.length===1?'package':'packages'}`:'Preview import'}</button>}
  </form>
  <p className="muted">Keep this window open until the upload finishes.</p>
  <button disabled={busy} onClick={()=>window.dispatchEvent(new Event('collector-open-work'))}>Open work &amp; history</button>
  </>}
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
   {detail.summary.permissions.can_correct_association&&<details><summary>Correct destination</summary>{target===undefined?<p role="status">Loading saved destination…</p>:<><DestinationChooser collectionId={collectionId} selected={target} disabled={busy} onSelect={value=>{targetEdited.current=true;setTarget(value);}}/>{associationChanged?<><p className="muted">This moves only this import.</p><button disabled={busy} onClick={()=>void correct()}>Save destination</button></>:<p role="status">{detail.import.association.item_id?'Already attached to this entry.':'Not attached to an entry.'}</p>}</>}</details>}
  </details>}
 </section>;
}
