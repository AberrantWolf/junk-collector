import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
type Plan=components['schemas']['IngestPlan'];
type Accepted=components['schemas']['AcceptedIngest'];
type Summary=components['schemas']['IngestSummary'];
type Detail=components['schemas']['IngestDetail'];
type Item=components['schemas']['PresentedItem'];
type InboxSource=components['schemas']['InboxSourceView'];
type Page=components['schemas']['Page_IngestSummary'];
type Props={collectionId:string;busy:boolean;setBusy:(value:boolean)=>void;onPublished:(item:Item)=>void;onError:(error:unknown)=>void};
function selectedName(files:File[]){return files.length===1?files[0].name:'Browser package';}
function fileName(file:File){return file.webkitRelativePath||file.name;}
function stateLabel(value:string){return value.replaceAll('_',' ');}

export default function GuidedIngest({collectionId,busy,setBusy,onPublished,onError}:Props){
 const [source,setSource]=useState<'browser'|'inbox'>('browser');
 const [files,setFiles]=useState<File[]>([]);const [resumeFiles,setResumeFiles]=useState<File[]>([]);
 const [packageName,setPackageName]=useState('Browser package');
 const [inboxes,setInboxes]=useState<InboxSource[]>([]);const [inboxLabel,setInboxLabel]=useState('');const [inboxPackages,setInboxPackages]=useState<string[]>([]);
 const [plan,setPlan]=useState<Plan>();const [automatic,setAutomatic]=useState(true);
 const [history,setHistory]=useState<Summary[]>([]);const [cursor,setCursor]=useState<string|null>();
 const [detail,setDetail]=useState<Detail>();const [items,setItems]=useState<Item[]>([]);const [target,setTarget]=useState('');
 const [notice,setNotice]=useState('');const [failure,setFailure]=useState('');
 const lifecycle=useRef(new AbortController());const selection=useRef<AbortController|undefined>(undefined);
 const acceptance=useRef<string|undefined>(undefined);const detailId=useRef<string|undefined>(undefined);
 const callbacks=useRef({onPublished,onError,setBusy});callbacks.current={onPublished,onError,setBusy};
 const notified=useRef(new Map<string,string>());
 const selectedInbox=inboxes.find(value=>value.label===inboxLabel);
 async function request<T=unknown>(path:string,init:RequestInit={}):Promise<T>{
  const signal=init.signal?AbortSignal.any([init.signal,lifecycle.current.signal]):lifecycle.current.signal;
  signal.throwIfAborted();const value=await api<T>(path,{...init,signal});signal.throwIfAborted();return value;
 }
 function fail(error:unknown){if(lifecycle.current.signal.aborted||(error instanceof DOMException&&error.name==='AbortError'))return;setFailure(requestErrorMessage(error));if(isApiRequestError(error)&&error.code==='unauthenticated')callbacks.current.onError(error);}
 async function mutate(action:()=>Promise<void>){setBusy(true);setFailure('');try{await action();}catch(error){fail(error);}finally{if(!lifecycle.current.signal.aborted)setBusy(false);}}
 async function refreshHistory(next?:string){
  const page=await request<Page>(`/ingests?collection_id=${collectionId}&limit=50${next?`&cursor=${encodeURIComponent(next)}`:''}`);
  setHistory(current=>{const map=new Map(current.map(value=>[value.import_id,value]));for(const value of page.items)map.set(value.import_id,value);return [...map.values()];});
  setCursor(current=>next||current===undefined?page.next_cursor:current);
 }
 async function publishNotice(value:Detail,signal?:AbortSignal){
  if(value.import.phase!=='published'||!value.summary.target_item_id)return;
  const item=await request<Item>(`/items/${value.summary.target_item_id}`,{signal});
  const marker=JSON.stringify(item);if(notified.current.get(item.id)!==marker){notified.current.set(item.id,marker);callbacks.current.onPublished(item);}
 }
 async function open(id:string){
  selection.current?.abort();const controller=new AbortController();selection.current=controller;detailId.current=id;
  try{
   const value=await request<Detail>(`/ingests/${id}`,{signal:controller.signal});setDetail(value);setTarget(value.import.association.item_id||'');
   let next:string|null|undefined;const choices:Item[]=[];
   do{const page=await request<{items:Item[];next_cursor?:string|null}>(`/items?collection_id=${collectionId}&limit=100${next?`&cursor=${encodeURIComponent(next)}`:''}`,{signal:controller.signal});choices.push(...page.items);next=page.next_cursor;}while(next&&choices.length<1000);
   setItems(choices);await publishNotice(value,controller.signal);
  }catch(error){fail(error);}
 }
 useEffect(()=>{
  lifecycle.current=new AbortController();let timer:ReturnType<typeof setTimeout>|undefined;
  async function poll(){try{await refreshHistory();const id=detailId.current;if(id){const value=await request<Detail>(`/ingests/${id}`);if(detailId.current===id){setDetail(value);await publishNotice(value,selection.current?.signal);}}}catch(error){fail(error);}finally{if(!lifecycle.current.signal.aborted)timer=setTimeout(poll,2000);}}
  void poll();void request<InboxSource[]>(`/collections/${collectionId}/inboxes`).then(value=>{setInboxes(value);setInboxLabel(value[0]?.label||'');}).catch(fail);
  return()=>{lifecycle.current.abort();selection.current?.abort();if(timer)clearTimeout(timer);callbacks.current.setBusy(false);};
 },[collectionId]);
 useEffect(()=>{setInboxPackages(selectedInbox?.packages.slice(0,1)||[]);},[selectedInbox?.label]);
 async function preview(){await mutate(async()=>{
  const browser=source==='browser';const value=await request<Plan>('/ingests/plan',{method:'POST',body:JSON.stringify({collection_id:collectionId,packages:browser?[{source:{kind:source,label:null,package_name:packageName},files:files.map(file=>({name:fileName(file),bytes:file.size,sha256:null}))}]:inboxPackages.map(name=>({source:{kind:'inbox',label:inboxLabel,package_name:name},files:[]}))})});setPlan(value);acceptance.current=crypto.randomUUID();
 });}
 async function upload(value:Detail,selectedFiles:File[],verifyPrefix:boolean){
  const selected=new Map(selectedFiles.map(file=>[fileName(file),file]));
  if(selected.size!==selectedFiles.length||selected.size!==value.import.files.length||value.import.files.some(entry=>selected.get(entry.name)?.size!==entry.bytes))throw new Error('Select the same complete package, with original file names and sizes.');
  for(let index=0;index<value.import.files.length;index++){
   const entry=value.import.files[index];const file=selected.get(entry.name)!;
   if(verifyPrefix)for(let offset=0;offset<entry.received_bytes;offset+=4*1024*1024)await request(`/imports/${value.summary.import_id}/files/${index}?offset=${offset}`,{method:'PUT',body:file.slice(offset,Math.min(offset+4*1024*1024,entry.received_bytes))});
  }
  for(let index=0;index<value.import.files.length;index++){
   const entry=value.import.files[index];const file=selected.get(entry.name)!;
   for(let offset=entry.received_bytes;offset<file.size;offset+=4*1024*1024)await request(`/imports/${value.summary.import_id}/files/${index}?offset=${offset}`,{method:'PUT',body:file.slice(offset,offset+4*1024*1024)});
  }
 }
 async function start(){if(!plan)return;const frozen=plan;const selected=[...files];await mutate(async()=>{
  acceptance.current??=crypto.randomUUID();
  const accepted=await request<Accepted>('/ingests',{method:'POST',headers:{'Idempotency-Key':acceptance.current},body:JSON.stringify({plan:frozen,enrichment:automatic?'automatic':'none'})});
  const summary=accepted.imports[0];if(!summary)throw new Error(accepted.rejected[0]?.reason||'No package was accepted.');
  setHistory(current=>[...current.filter(value=>!accepted.imports.some(next=>next.import_id===value.import_id)),...accepted.imports]);
  detailId.current=summary.import_id;const current=await request<Detail>(`/ingests/${summary.import_id}`);setDetail(current);setPlan(undefined);
  if(frozen.packages[0].source.kind==='browser'&&current.import.phase==='staging')await upload(current,selected,false);
  await open(summary.import_id);await refreshHistory();setFiles([]);setNotice('Import accepted. The server continues after all package bytes arrive.');
 });}
 async function resume(){if(!detail)return;await mutate(async()=>{const current=await request<Detail>(`/ingests/${detail.summary.import_id}`);if(current.import.phase!=='staging')throw new Error('This import no longer needs an upload. Refresh its detail.');await upload(current,resumeFiles,true);setResumeFiles([]);await open(current.summary.import_id);await refreshHistory();});}
 async function discard(){if(!detail||!confirm(`Discard ${detail.summary.package_name} and reclaim its unpublished staging bytes?`))return;await mutate(async()=>{await request(`/ingests/${detail.summary.import_id}/discard`,{method:'POST',body:JSON.stringify({expected_revision:detail.import.revision})});setNotice('Discard accepted. Cleanup progress is shown below.');await open(detail.summary.import_id);await refreshHistory();});}
 async function cancel(){if(!detail)return;await mutate(async()=>{setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/cancel`,{method:'POST',body:JSON.stringify({expected_revision:detail.import.revision})}));await refreshHistory();});}
 async function retry(jobId?:string|null){if(!detail)return;await mutate(async()=>{setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/retry`,{method:'POST',body:JSON.stringify({job_id:jobId||null})}));await refreshHistory();});}
 async function correct(){if(!detail||detail.summary.capture_revision===null)return;await mutate(async()=>{
  const selected=items.find(value=>value.id===target);if(target&&!selected)throw new Error('Select an available target entry before saving.');
  setDetail(await request<Detail>(`/ingests/${detail.summary.import_id}/association`,{method:'PUT',body:JSON.stringify({expected_capture_revision:detail.summary.capture_revision,association:{item_id:selected?.id||null,specimen_id:null,component_id:null,carrier_id:null},expected_item_revision:selected?.revision||null})}));await refreshHistory();if(selected)callbacks.current.onPublished(await request<Item>(`/items/${selected.id}`));
 });}
 return <section className="guided-ingest" aria-labelledby="guided-ingest-heading"><div className="section-heading"><div><h3 id="guided-ingest-heading">Import original packages</h3><p className="muted">Preview package boundaries, preserve the source, and let the server continue identification and optional metadata after this window closes.</p></div></div>
  {failure&&<p role="alert" className="notice error">{failure}</p>}{notice&&<p role="status" className="notice">{notice}</p>}<form onSubmit={event=>{event.preventDefault();void (plan?start():preview());}}><fieldset className="source-choice"><legend>Package source</legend><label><input type="radio" name="guided-source" disabled={busy} checked={source==='browser'} onChange={()=>{setSource('browser');setPlan(undefined);}}/> This browser</label><label><input type="radio" name="guided-source" disabled={busy} checked={source==='inbox'} onChange={()=>{setSource('inbox');setPlan(undefined);}}/> Configured inbox</label></fieldset>{source==='browser'?<><label>Package name<input value={packageName} disabled={busy} onChange={event=>{setPackageName(event.target.value);setPlan(undefined);}}/></label><label>Package files<input aria-label="Package files" type="file" multiple disabled={busy} onChange={event=>{const selected=Array.from(event.target.files||[]);setFiles(selected);setPackageName(current=>current==='Browser package'?selectedName(selected):current);setPlan(undefined);}}/></label>{files.length>0&&<div className="package-files compact" role="list">{files.map(file=><div role="listitem" key={fileName(file)}><span>{fileName(file)}</span><small>{file.size.toLocaleString()} bytes</small></div>)}</div>}</>:inboxes.length?<div className="form-grid"><label>Inbox<select value={inboxLabel} disabled={busy} onChange={event=>{setInboxLabel(event.target.value);setPlan(undefined);}}>{inboxes.map(value=><option key={value.label}>{value.label}</option>)}</select></label><label>Packages<select multiple aria-label="Inbox packages" value={inboxPackages} disabled={busy} onChange={event=>{setInboxPackages(Array.from(event.target.selectedOptions,option=>option.value));setPlan(undefined);}}>{selectedInbox?.packages.map(value=><option key={value}>{value}</option>)}</select></label></div>:<p className="notice">No configured inbox packages are available. Choose files from this browser.</p>}{plan&&<div className="ingest-preview"><h4>Preview</h4>{plan.packages.map(pkg=><div key={pkg.position}><strong>{pkg.default_title}</strong><p>{pkg.files.length} files · destination: new collection entry</p>{pkg.anchors.map(anchor=><p className="muted" key={anchor.file_index}>{anchor.selected?'Selected':'Eligible'} {anchor.label} anchor: {pkg.files[anchor.file_index]?.name}</p>)}{pkg.blockers.map(blocker=><p className="notice" key={`${blocker.kind}:${blocker.message}`}>{blocker.message}</p>)}<p className="muted">Metadata: {pkg.metadata_available?'available':'unavailable or not configured'} · Artwork: {pkg.artwork_available?'available':'unavailable or not configured'}</p></div>)}<label><input type="checkbox" checked={automatic} onChange={event=>setAutomatic(event.target.checked)}/> Automatically gather available metadata and artwork</label></div>}<button className="primary" disabled={busy||(source==='browser'?(!files.length||!packageName.trim()):!inboxLabel||!inboxPackages.length)}>{plan?'Start import':'Preview import'}</button></form>
  <h4>Import history</h4>{history.length===0?<p className="muted">No guided imports yet.</p>:<div className="ingest-history">{history.map(value=><button type="button" key={value.import_id} onClick={()=>void open(value.import_id)}><strong>{value.package_name}</strong><span>{stateLabel(value.phase)} · {stateLabel(value.outcome)} · {value.file_count} files</span><small>{new Date(value.updated_at).toLocaleString()} · {value.import_id}</small></button>)}</div>}
  {cursor&&<button onClick={()=>void refreshHistory(cursor).catch(fail)}>More import history</button>}
  {detail&&<div className="ingest-detail"><div className="section-heading"><div><p className="eyebrow">Import detail</p><h4>{detail.summary.package_name}</h4></div><button onClick={()=>{selection.current?.abort();detailId.current=undefined;setDetail(undefined);}}>Close</button></div><p className="muted">Import {detail.summary.import_id}<br/>Capture {detail.summary.capture_id}</p>{detail.cleanup&&<p role="status">Reclaimed {detail.cleanup.reclaimed_bytes.toLocaleString()} bytes · {detail.cleanup.pending_bytes.toLocaleString()} pending{detail.cleanup.cleanup_pending?" · cleanup pending":" · cleanup complete"}</p>}<div className="ingest-steps">{detail.steps.map(step=><div key={step.step}><strong>{stateLabel(step.step)}</strong><span className={`identity ${step.state}`}>{stateLabel(step.state)}</span>{step.attempt&&<small>attempt {step.attempt}</small>}{step.message&&<p>{step.message}</p>}{step.retryable&&<button disabled={busy} onClick={()=>void retry(step.job_id)}>Retry this step</button>}</div>)}</div>{detail.import.phase==='staging'&&detail.import.source.kind==='browser'&&<form aria-label="Resume guided package" onSubmit={event=>{event.preventDefault();void resume();}}><p className="notice">Select the same complete package to resume. Retained prefixes are verified before new bytes are appended.</p><label>Original package files<input aria-label="Resume original package files" type="file" multiple disabled={busy} onChange={event=>setResumeFiles(Array.from(event.target.files||[]))}/></label><button className="primary" disabled={busy||!resumeFiles.length}>Resume upload</button></form>}{detail.attempts.length>0&&<details><summary>Attempt history</summary>{detail.attempts.map((attempt,index)=><p key={`${attempt.job_id}:${attempt.attempt}:${index}`}><strong>{stateLabel(attempt.step)} · attempt {attempt.attempt}</strong> — {stateLabel(attempt.state)}{attempt.diagnostic&&<> · {attempt.diagnostic}</>}</p>)}</details>}{detail.summary.permissions.can_cancel&&<button disabled={busy} onClick={()=>void cancel()}>Cancel remaining work</button>}{detail.summary.permissions.can_discard&&<button className="danger" disabled={busy} onClick={()=>void discard()}>Discard unpublished import</button>}{detail.summary.outcome==='failed'&&!detail.steps.some(step=>step.retryable)&&<button disabled={busy} onClick={()=>void retry(null)}>Retry optional enrichment</button>}{detail.summary.permissions.can_correct_association&&<div className="association-correction"><label>Published association<select value={target} onChange={event=>setTarget(event.target.value)}><option value="">Leave capture unattached</option>{items.map(item=><option key={item.id} value={item.id}>{item.title}</option>)}</select></label><button disabled={busy} onClick={()=>void correct()}>Correct association</button><p className="muted">This changes only the Capture association. Original files and evidence are retained.</p></div>}</div>}
 </section>;
}
