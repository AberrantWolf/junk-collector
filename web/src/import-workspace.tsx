import React,{useEffect,useState,api,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type ImportView=components['schemas']['ImportView'];
type InboxSource=components['schemas']['InboxSourceView'];
type JobView=components['schemas']['JobView'];
type Association=components['schemas']['CaptureAssociation'];
type Props={item:Item;busy:boolean;setBusy:(value:boolean)=>void;onError:(error:unknown)=>void};

const terminal=new Set<JobView['state']>(['completed','failed','cancelled']);
const phaseLabel:Record<ImportView['phase'],string>={staging:'Staging package',analyzing:'Ready for review',awaiting_review:'Review recorded',publishing:'Publishing capture',published:'Capture published',cancelled:'Import cancelled'};

function fileName(file:File){return file.webkitRelativePath||file.name;}
function packageName(files:File[]){const first=files[0]&&fileName(files[0]);return first?.includes('/')?first.split('/')[0]:'Browser package';}
function percent(done:number,total:number){return total?Math.min(100,Math.round(done/total*100)):100;}
function jobsChanged(){window.dispatchEvent(new Event('collector-jobs-updated'));}

async function waitForJob(id:string,onProgress:(job:JobView)=>void){
 for(;;){
  const job=await api<JobView>(`/jobs/${id}`);onProgress(job);
  if(terminal.has(job.state)){
   if(job.state==='failed')throw new Error(job.error||job.limitation||'The background job failed.');
   if(job.state==='cancelled')throw new Error('The background job was cancelled.');
   return job;
  }
  await new Promise(resolve=>setTimeout(resolve,2000));
 }
}

export default function ImportWorkspace({item,busy,setBusy,onError}:Props){
 const [source,setSource]=useState<'browser'|'inbox'>('browser');
 const [files,setFiles]=useState<File[]>([]);const [inboxes,setInboxes]=useState<InboxSource[]>([]);
 const [label,setLabel]=useState('');const [inboxPackage,setInboxPackage]=useState('');
 const [record,setRecord]=useState<ImportView>();const [job,setJob]=useState<JobView>();
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

 useEffect(()=>{let active=true;api<InboxSource[]>(`/collections/${item.collection_id}/inboxes`).then(value=>{if(!active)return;setInboxes(value);setLabel(current=>current||value[0]?.label||'');}).catch(onError);return()=>{active=false;};},[item.collection_id]);
 useEffect(()=>{const id=localStorage.getItem(storageKey);if(id)api<ImportView>(`/imports/${id}`).then(setRecord).catch(error=>{localStorage.removeItem(storageKey);onError(error);});},[storageKey]);
 useEffect(()=>{if(record)localStorage.setItem(storageKey,record.id);},[record?.id,storageKey]);
 useEffect(()=>{setItemRevision(item.revision);},[item.revision]);
 useEffect(()=>{setInboxPackage(selectedInbox?.packages[0]||'');},[selectedInbox?.label]);
 useEffect(()=>{setComponent('');setCarrier('');},[specimen]);
 useEffect(()=>{setCarrier('');},[component]);
 useEffect(()=>{const refresh=()=>{if(record)api<ImportView>(`/imports/${record.id}`).then(setRecord).catch(onError);};window.addEventListener('online',refresh);window.addEventListener('collector-jobs-updated',refresh);return()=>{window.removeEventListener('online',refresh);window.removeEventListener('collector-jobs-updated',refresh);};},[record?.id]);

 function reset(){localStorage.removeItem(storageKey);setRecord(undefined);setJob(undefined);setConflict('');setFiles([]);setIdentity('unidentified');setAttach(true);setSpecimen('');setComponent('');setCarrier('');}
 async function stage(){
  setBusy(true);setJob(undefined);
  try{
   if(source==='browser'){
    const created=await api<ImportView>('/imports',{method:'POST',body:JSON.stringify({collection_id:item.collection_id,package_name:packageName(files),files:files.map(file=>({name:fileName(file),bytes:file.size,sha256:null}))})});
    setRecord(created);let current=created;
    for(let index=0;index<files.length;index++){
     const file=files[index];
     for(let offset=current.files[index].received_bytes;offset<file.size;offset+=4*1024*1024){
      current=await api<ImportView>(`/imports/${created.id}/files/${index}?offset=${offset}`,{method:'PUT',body:file.slice(offset,offset+4*1024*1024)});setRecord(current);
     }
    }
   }else{
    const staged=await api<{import:ImportView;job_id:string}>(`/collections/${item.collection_id}/inboxes/${encodeURIComponent(label)}/stage`,{method:'POST',body:JSON.stringify({package:inboxPackage})});
    setRecord(staged.import);jobsChanged();await waitForJob(staged.job_id,setJob);jobsChanged();setRecord(await api<ImportView>(`/imports/${staged.import.id}`));
   }
  }catch(error){onError(error);}finally{setBusy(false);}
 }
 async function beginReview(){
  if(!record)return;setBusy(true);setJob(undefined);
  try{
   const accepted=await api<{job_id:string}>(`/imports/${record.id}/analysis`,{method:'POST',body:JSON.stringify({module:item.domain.module})});
   jobsChanged();await waitForJob(accepted.job_id,setJob);jobsChanged();setRecord(await api<ImportView>(`/imports/${record.id}`));
  }catch(error){onError(error);}finally{setBusy(false);}
 }
 function association():Association{
  if(!attach)return {item_id:null,specimen_id:null,component_id:null,carrier_id:null};
  return {item_id:item.id,specimen_id:specimen||null,component_id:component||null,carrier_id:carrier||null};
 }
 async function saveReview(){
  if(!record)return;setBusy(true);
  try{
   const choice=identity==='unidentified'?{identity:'unidentified'}:{identity:'candidate',key:identity};
   const reviewed=await api<ImportView>(`/imports/${record.id}/review`,{method:'PUT',body:JSON.stringify({expected_revision:record.revision,identity:choice,association:association(),expected_item_revision:attach?itemRevision:null})});setRecord(reviewed);setConflict('');
  }catch(error){
   if(isApiRequestError(error)&&error.status===409){
    const [current,currentItem]=await Promise.all([api<ImportView>(`/imports/${record.id}`),api<Item>(`/items/${item.id}`)]);setRecord(current);setItemRevision(currentItem.revision);setConflict(`The server is now at Import revision ${current.revision} and Item revision ${currentItem.revision}. Your review choices are still in this form; review them and apply again.`);
   }else onError(error);
  }finally{setBusy(false);}
 }
 async function publish(){
  if(!record)return;setBusy(true);setJob(undefined);
  try{
   const accepted=await api<{job_id:string}>(`/imports/${record.id}/publish`,{method:'POST'});
   jobsChanged();await waitForJob(accepted.job_id,setJob);jobsChanged();setRecord(await api<ImportView>(`/imports/${record.id}`));
  }catch(error){onError(error);}finally{setBusy(false);}
 }

 const canStage=source==='browser'?files.length>0:!!label&&!!inboxPackage;
 return <section className="import-workspace" aria-labelledby="import-heading">
  <div className="section-heading"><div><h3 id="import-heading">Capture accession</h3><p className="muted">Stage original package bytes, inspect the record, then choose what—if anything—they belong to.</p></div>{record&&<button disabled={busy} onClick={reset}>Stage another package</button>}</div>
  {!record&&<form aria-label="Stage package" onSubmit={event=>{event.preventDefault();void stage();}}>
   <fieldset className="source-choice"><legend>Package source</legend><label><input type="radio" name="import-source" checked={source==='browser'} onChange={()=>setSource('browser')}/> This browser</label><label><input type="radio" name="import-source" checked={source==='inbox'} onChange={()=>setSource('inbox')}/> Configured inbox</label></fieldset>
   {source==='browser'?<label>Original package files<input aria-label="Original package files" type="file" disabled={busy} multiple onChange={event=>setFiles(Array.from(event.target.files||[]))}/></label>:inboxes.length?<div className="form-grid"><label>Inbox<select value={label} disabled={busy} onChange={event=>setLabel(event.target.value)}>{inboxes.map(value=><option key={value.label}>{value.label}</option>)}</select></label><label>Package<select value={inboxPackage} disabled={busy} onChange={event=>setInboxPackage(event.target.value)}>{selectedInbox?.packages.map(value=><option key={value}>{value}</option>)}</select></label></div>:<p className="notice">No configured inbox packages are available. Choose files from this browser.</p>}
   {source==='browser'&&files.length>0&&<div className="package-files compact" role="list">{files.map(file=><div role="listitem" key={fileName(file)}><span>{fileName(file)}</span><small>{file.size.toLocaleString()} bytes</small></div>)}</div>}
   <button className="primary" disabled={busy||!canStage}>Stage package</button>
  </form>}
  {record&&<div className="import-docket">
   <header><div><span className="docket-phase">{phaseLabel[record.phase]}</span><strong className="docket-title">{record.source.package_name}</strong></div><span>{percent(done,total)}%</span></header>
   <progress max={Math.max(total,1)} value={total?done:1} aria-label="Package staging progress"/><p className="muted" role="status" aria-live="polite">{job?`${job.phase}: ${percent(job.completed_units,job.total_units)}%`:`${done.toLocaleString()} of ${total.toLocaleString()} bytes staged`}</p>
   <h4>Package inventory</h4><div className="package-files" role="list">{record.files.map(file=><div role="listitem" key={file.name}><span>{file.name}</span><small>{file.received_bytes.toLocaleString()} / {file.bytes.toLocaleString()} bytes · {file.digest?'SHA-256 recorded':'digest pending'}</small></div>)}</div>
   {record.limitations.length>0&&<div className="import-limitations"><h4>Analysis limitations</h4><ul>{record.limitations.map(value=><li key={value}>{value}</li>)}</ul></div>}
   {record.phase==='analyzing'&&<button className="primary" disabled={busy} onClick={()=>void beginReview()}>Review import</button>}
   {conflict&&<p className="notice conflict" role="alert"><strong>Revision conflict.</strong> {conflict}</p>}
   {record.phase==='awaiting_review'&&(!record.review||!!conflict)&&<form aria-label="Review import" onSubmit={event=>{event.preventDefault();void saveReview();}}>
    <fieldset><legend>Identity</legend><label><input type="radio" name="identity" value="unidentified" checked={identity==='unidentified'} onChange={event=>setIdentity(event.target.value)}/> Keep unidentified</label>{candidates.map(candidate=><label key={candidate.key}><input type="radio" name="identity" value={candidate.key} checked={identity===candidate.key} onChange={event=>setIdentity(event.target.value)}/> {candidate.label}</label>)}</fieldset>
    <fieldset><legend>Physical association</legend><label><input type="radio" name="attachment" checked={attach} onChange={()=>setAttach(true)}/> Attach to {item.title}</label><label><input type="radio" name="attachment" checked={!attach} onChange={()=>setAttach(false)}/> Leave capture unattached</label></fieldset>
    {attach&&item.specimens.length>0&&<div className="form-grid"><label>Physical copy<select value={specimen} onChange={event=>setSpecimen(event.target.value)}><option value="">Item only</option>{item.specimens.map(value=><option key={value.id} value={value.id}>{value.label}</option>)}</select></label>{selectedSpecimen&&<label>Part<select value={component} onChange={event=>setComponent(event.target.value)}><option value="">Copy only</option>{selectedSpecimen.components.map(value=><option key={value.id} value={value.id}>{value.position}. {value.label}</option>)}</select></label>}{selectedComponent?.carrier&&<label>Carrier<select value={carrier} onChange={event=>setCarrier(event.target.value)}><option value="">Part only</option><option value={selectedComponent.carrier.id}>{selectedComponent.carrier.label}</option></select></label>}</div>}
    <button className="primary" disabled={busy}>Review import</button>
   </form>}
   {record.phase==='awaiting_review'&&record.review&&!conflict&&<div className="review-decision"><p><strong>Review saved.</strong> {record.review.identity.identity==='unidentified'?'No catalog identity was asserted.':'A candidate was selected for this capture.'}</p><button className="primary" disabled={busy} onClick={()=>void publish()}>Publish capture</button></div>}
   {record.phase==='published'&&<p className="notice" role="status">Capture published. The original package and integrity evidence are now in the archive.</p>}
  </div>}
 </section>;
}
