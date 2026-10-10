import {presentationLabel} from './presentation-labels';
import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import {useDialogDismiss} from './dialog';
import GuidedIngest from './guided-ingest';
import ImportWorkspace from './import-workspace';
import {uploadPackage,type UploadProgress} from './package-upload';
import {UploadMeter} from './work-progress';
import {bytes} from './work-progress';
import DestinationChooser from './destination-chooser';
import {CollectionChooser,useCollectionSearch} from './collection-chooser';
type Entry=components['schemas']['WorkEntry'];
function workName(entry:Entry){return entry.target.kind==='job'?presentationLabel(entry.kind,'Background task'):entry.name;}
type Target=components['schemas']['WorkTarget'];
type Action=components['schemas']['WorkHistoryAction'];

type Item=components['schemas']['PresentedItem'];
type Collection=components['schemas']['PresentedCollection'];
const key=(v:Target)=>`${v.kind}:${v.id}`;
const label:Record<Action,string>={dismiss:'Dismiss history',restore:'Restore history',discard:'Discard unfinished import',cancel_and_clear:'Cancel and clear unfinished work',retry_cleanup:'Retry cleanup'};
export default function WorkHistory({collections,onClose,onSessionExpired,initialCollectionId,initialItemId,onSaved,embedded=false}:{collections:Collection[];onClose:()=>void;onSessionExpired:()=>void;initialCollectionId?:string;initialItemId?:string;onSaved:(item:Item)=>void;embedded?:boolean}){
 const dialog=useDialogDismiss(onClose,!embedded);const [collection,setCollection]=useState(initialCollectionId||'');const [itemId,setItemId]=useState(initialItemId||'');
 const collectionChoices=useCollectionSearch(collection,{},collections.find(value=>value.id===collection),onSessionExpired);
 const [search,setSearch]=useState('');const [status,setStatus]=useState('');const [archived,setArchived]=useState(false);const [unattached,setUnattached]=useState(false);const [after,setAfter]=useState('');const [before,setBefore]=useState('');
 const [entries,setEntries]=useState<Entry[]>([]);const [cursor,setCursor]=useState<string|null>();const loadedPages=useRef(1);const oldestLoaded=useRef<{createdAt:number;key:string}|undefined>(undefined);
 const [selected,setSelected]=useState<string[]>([]);const [review,setReview]=useState<{action:Action;entries:Entry[];keys:string[]}>();const [outcomes,setOutcomes]=useState<string[]>([]);const [changing,setChanging]=useState(false);const [loading,setLoading]=useState(false);const [error,setError]=useState('');const [detail,setDetail]=useState<Entry>();const [detailBusy,setDetailBusy]=useState(false);
 useEffect(()=>{setSelected(current=>current.filter(id=>entries.some(e=>key(e.target)===id)));},[entries]);
 const detailEntryRef=useRef<Entry|undefined>(undefined);detailEntryRef.current=detail;
 const detailRef=useRef<HTMLElement>(null);const reviewRef=useRef<HTMLElement>(null);const listScroll=useRef(0);const fromList=useRef(false);
 useEffect(()=>{if(detail){if(dialog.ref.current)dialog.ref.current.scrollTop=0;detailRef.current?.focus({preventScroll:true});}else if(dialog.ref.current)dialog.ref.current.scrollTop=listScroll.current;},[detail?.target.id]);
 useEffect(()=>{if(review){reviewRef.current?.scrollIntoView({block:'nearest'});reviewRef.current?.focus({preventScroll:true});}},[review]);
 const lifecycle=useRef(new AbortController());const refresh=useRef<()=>Promise<void>>(async()=>{});
 function fail(e:unknown){if(e instanceof DOMException&&e.name==='AbortError')return;if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}
 
 const scope=JSON.stringify([collection,itemId,search,status,archived,unattached,after,before]);
 useEffect(()=>{
  const controller=new AbortController();lifecycle.current=controller;let timer:ReturnType<typeof setTimeout>;let inFlight:Promise<void>|undefined;
  loadedPages.current=1;oldestLoaded.current=undefined;setEntries([]);setSelected([]);setReview(undefined);setCursor(undefined);
  function query(cursor?:string){const params=new URLSearchParams({limit:'25',archived:String(archived),unattached:String(unattached)});if(collection)params.set('collection_id',collection);if(itemId)params.set('item_id',itemId);if(search)params.set('search',search);if(status)params.set('status',status);if(after)params.set('after',String(Math.floor(new Date(after+'T00:00:00').getTime()/1000)));if(before)params.set('before',String(Math.floor(new Date(before+'T23:59:59').getTime()/1000)));if(cursor)params.set('cursor',cursor);return params;}
  async function read(){try{
   const values:Entry[]=[];let next:string|null|undefined;let pages=0;
   for(;;){
    const page=await api<components['schemas']['Page_WorkEntry']>(`/work?${query(next||undefined)}`,{signal:controller.signal});
    values.push(...page.items);next=page.next_cursor;pages++;
    const tail=values.at(-1);const boundary=oldestLoaded.current;
    const reachedBoundary=!!tail&&!!boundary&&(tail.created_at<boundary.createdAt||(tail.created_at===boundary.createdAt&&key(tail.target)<=boundary.key));
    if(!next||!page.items.length||(pages>=loadedPages.current&&(!boundary||reachedBoundary)))break;
   }
   if(controller.signal.aborted)return;
   loadedPages.current=Math.max(loadedPages.current,pages);
   const tail=values.at(-1);if(tail&&(!oldestLoaded.current||tail.created_at<oldestLoaded.current.createdAt||(tail.created_at===oldestLoaded.current.createdAt&&key(tail.target)<oldestLoaded.current.key)))oldestLoaded.current={createdAt:tail.created_at,key:key(tail.target)};
   setEntries(values);setCursor(next||null);
   const opened=detailEntryRef.current;
   if(opened){try{const current=await api<Entry>('/work/entry',{method:'POST',body:JSON.stringify(opened.target),signal:controller.signal});if(!controller.signal.aborted&&detailEntryRef.current&&key(detailEntryRef.current.target)===key(opened.target))setDetail(current);}catch(e){if(isApiRequestError(e)&&[403,404].includes(e.status))setDetail(undefined);throw e;}}
   setError('');
  }catch(e){if(!controller.signal.aborted)fail(e);}finally{if(!controller.signal.aborted)setLoading(false);}}
  async function poll(force=false){if(inFlight){if(!force)return inFlight;await inFlight;}const current=read();inFlight=current;try{await current;}finally{if(inFlight===current)inFlight=undefined;}}
  refresh.current=()=>poll(true);setLoading(true);const start=setTimeout(()=>void poll(),180);
  const schedule=async()=>{await poll();if(!controller.signal.aborted)timer=setTimeout(schedule,5000);};timer=setTimeout(schedule,5000);
  return()=>{controller.abort();clearTimeout(start);clearTimeout(timer);};
 },[scope]);
 async function more(){if(!cursor||loading)return;loadedPages.current++;setLoading(true);await refresh.current();}
 async function open(e:Entry){listScroll.current=dialog.ref.current?.scrollTop||0;fromList.current=true;const url=new URL(location.href);url.searchParams.set('work',key(e.target));history.pushState({...history.state},'',url);setDetail(e);}
 function back(){if(fromList.current){history.back();return;}const url=new URL(location.href);url.searchParams.set('work','list');history.replaceState({...history.state},'',url);setDetail(undefined);}
 useEffect(()=>{let controller=new AbortController();const read=async()=>{controller.abort();controller=new AbortController();const signal=controller.signal;const value=new URL(location.href).searchParams.get('work');if(!value){onClose();return;}if(value==='list'){setDetail(undefined);return;}const [kind,id]=value.split(':');if(!['import','job'].includes(kind)||!id){setError('Invalid work link.');return;}try{const entry=await api<Entry>('/work/entry',{method:'POST',body:JSON.stringify({kind,id}),signal});if(!signal.aborted)setDetail(entry);}catch(e){if(!signal.aborted){fail(e);setDetail(undefined);}}};const pop=()=>void read();window.addEventListener('popstate',pop);void read();return()=>{controller.abort();window.removeEventListener('popstate',pop);};},[]);
 async function preview(action:Action){setError('');try{const values=await Promise.all(entries.filter(e=>selected.includes(key(e.target))).map(e=>api<Entry>('/work/entry',{method:'POST',body:JSON.stringify(e.target),signal:lifecycle.current.signal})));setReview({action,entries:values,keys:values.map(()=>crypto.randomUUID())});setOutcomes([]);}catch(e){fail(e);}}
 async function apply(){
  if(!review)return;setChanging(true);const results:string[]=[];const changed=new Set<string>();
  try{
   for(let i=0;i<review.entries.length;i++){
    const e=review.entries[i];if(!(e.actions||[]).includes(review.action)){results.push(`${workName(e)}: not eligible; unchanged`);continue;}
    try{await api('/work',{method:'POST',headers:{'Idempotency-Key':review.keys[i]},body:JSON.stringify({target:e.target,action:review.action}),signal:lifecycle.current.signal});changed.add(key(e.target));results.push(`${workName(e)}: ${['dismiss','restore'].includes(review.action)?'updated':'requested; progress appears in the list'}`);}
    catch(err){results.push(`${workName(e)}: ${requestErrorMessage(err)}`);}
   }
   if(changed.size)setEntries(current=>current.filter(entry=>!changed.has(key(entry.target))));
   setOutcomes(results);setSelected([]);await refresh.current();
  }finally{setChanging(false);}
 }
 return <div className={embedded?'activity-history':'modal-backdrop'}><section ref={dialog.ref} className={embedded?'work-history':'modal curation-modal work-history'} role={embedded?undefined:'dialog'} aria-modal={embedded?undefined:true} aria-labelledby="work-heading" aria-busy={changing}>{!embedded&&<button className="close" aria-label="Close work history" onClick={dialog.dismiss}>×</button>}<h2 id="work-heading">Work &amp; history</h2>{!detail?<><p>Review your imports and background work. Dismissing history keeps original files and successful results.</p>
 <fieldset disabled={changing}><legend>Find work</legend><div className="form-grid"><label>Search work<input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Package, file or entry name"/></label><CollectionChooser label="Collection" value={collection} onChange={value=>{setCollection(value);setItemId('');}} choices={collectionChoices} all/><label>Status<select value={status} onChange={e=>setStatus(e.target.value)}><option value="">All statuses</option>{['active','needs_input','completed','failed','cancelled'].map(s=><option key={s} value={s}>{presentationLabel(s)}</option>)}</select></label></div><label className="work-toggle"><input type="checkbox" checked={archived} onChange={e=>setArchived(e.target.checked)}/> Show dismissed history</label><details><summary>Date and attachment filters</summary><div className="form-grid"><label>From<input type="date" value={after} onChange={e=>setAfter(e.target.value)}/></label><label>Through<input type="date" value={before} onChange={e=>setBefore(e.target.value)}/></label></div><label className="work-toggle"><input type="checkbox" checked={unattached} onChange={e=>setUnattached(e.target.checked)}/> Tasks without an entry</label></details>{itemId&&<button onClick={()=>setItemId('')}>Show work for all entries</button>}
 </fieldset><button disabled={loading} onClick={()=>void refresh.current()}>Refresh work</button>{error&&<p role="alert" className="notice error">{error}</p>}
 <div className="row-actions"><span>{selected.length} selected</span><button disabled={!entries.length||changing} onClick={()=>setSelected(entries.map(e=>key(e.target)))}>Select loaded work</button><button disabled={!selected.length||changing} onClick={()=>setSelected([])}>Clear selection</button>{(archived?['restore']:['dismiss','discard','cancel_and_clear','retry_cleanup']).map(action=><button key={action} disabled={!selected.length||changing} onClick={()=>void preview(action as Action)}>{label[action as Action]}</button>)}</div>
 {review&&<section ref={reviewRef} tabIndex={-1} className="work-review" aria-label="Review selected work changes"><h3>{label[review.action]}</h3><p>Only these {review.entries.length} selected entries are included. Original files and successful results are kept. Temporary files are removed after cancellation finishes.</p><ul>{review.entries.map(e=><li key={key(e.target)}>{workName(e)} · {e.item_title||'Unattached'} · {presentationLabel(e.status)} — {(e.actions||[]).includes(review.action)?`Eligible${e.target.kind==='import'&&e.phase!=='published'&&['discard','cancel_and_clear','retry_cleanup'].includes(review.action)?` · up to ${bytes(e.received_bytes)} staging`:' · no published bytes reclaimed'}`:'Not eligible; will be skipped'}</li>)}</ul><button disabled={changing||!review.entries.some(e=>(e.actions||[]).includes(review.action))} onClick={()=>void apply()}>Apply to eligible selected work</button><button disabled={changing} onClick={()=>setReview(undefined)}>Close review</button>{outcomes.length>0&&<ul role="status">{outcomes.map((s,i)=><li key={i}>{s}</li>)}</ul>}</section>}
 {loading&&!entries.length&&<p role="status">Loading work…</p>}{!loading&&!entries.length&&<p>No matching work.</p>}
 <div className="job-list">{entries.map(e=><article key={key(e.target)}><header><label><input aria-label={`Select ${workName(e)}`} type="checkbox" checked={selected.includes(key(e.target))} disabled={changing} onChange={v=>setSelected(current=>v.target.checked?[...current,key(e.target)]:current.filter(k=>k!==key(e.target)))}/><strong>{workName(e)}</strong></label><span>{presentationLabel(e.status)}</span></header><p>{e.collection_name} · {e.item_title||'Unattached'} · {presentationLabel(e.phase)}</p><small>Submitted {new Date(e.created_at*1000).toLocaleString()}</small>{e.clear_pending&&<p role="status">Clearing history after cancellation and cleanup finish…</p>}{(e.reclaimed_bytes>0||e.pending_bytes>0)&&<p>Temporary files cleared: {bytes(e.reclaimed_bytes)} · Remaining: {bytes(e.pending_bytes)}</p>}{(e.clear_error||e.cleanup_error)&&<p role="alert" className="error">{e.clear_error||e.cleanup_error}</p>}{e.shared&&<p>This task is also needed elsewhere and will continue.</p>}<button data-work-open={key(e.target)} onClick={()=>void open(e)}>Open work</button></article>)}</div>{cursor&&<button disabled={loading} onClick={()=>void more()}>Load older work</button>}
 </>:<section ref={detailRef} tabIndex={-1} aria-label="Selected work"><button className="back-to-results" onClick={back}>Back to history</button><h3>{workName(detail)}</h3><p>{detail.collection_name} · {detail.item_title||'Unattached'} · {presentationLabel(detail.status)}</p>{error&&<p role="alert" className="notice error">{error}</p>}{detail.target.kind==='import'&&detail.guided&&detail.collection_id?<GuidedIngest key={`guided:${detail.target.id}`} initialImportId={detail.target.id} historyOnly collectionId={detail.collection_id} busy={detailBusy} setBusy={setDetailBusy} onPublished={onSaved} onError={fail}/>:detail.target.kind==='import'?<ManualImportWork key={`manual:${detail.target.id}`} entry={detail} onSaved={onSaved} onError={fail}/>:null}<WorkExecutions key={`executions:${detail.target.id}`} entry={detail} onError={fail}/></section>}
 </section></div>;
}

function ManualImportWork({entry,onSaved,onError}:{entry:Entry;onSaved:(item:Item)=>void;onError:(e:unknown)=>void}){
 const [record,setRecord]=useState<components['schemas']['ImportView']>();const [files,setFiles]=useState<File[]>([]);const [upload,setUpload]=useState<UploadProgress>();const controller=useRef(new AbortController());
 useEffect(()=>{const current=new AbortController();controller.current=current;void api<components['schemas']['ImportView']>(`/imports/${entry.target.id}`,{signal:current.signal}).then(setRecord).catch(e=>{if(!current.signal.aborted)onError(e);});return()=>current.abort();},[entry.target.id]);
 async function resume(){if(!record)return;try{await uploadPackage(record,files,true,controller.current.signal,setUpload,setRecord);setFiles([]);}catch(e){if(!controller.current.signal.aborted)onError(e);}finally{if(!controller.current.signal.aborted)setUpload(undefined);}}
 const [item,setItem]=useState<Item|null>(null);
 useEffect(()=>{if(!entry.item_id)return;const controller=new AbortController();void api<Item>(`/items/${entry.item_id}`,{signal:controller.signal}).then(setItem).catch(e=>{if(!controller.signal.aborted)onError(e);});return()=>controller.abort();},[entry.item_id]);
 return <div data-uploading={!!upload}>{record&&<><p>{presentationLabel(record.phase)} · {record.files.length} {record.files.length===1?'file':'files'}</p><details><summary>Files</summary><ul>{record.files.map(f=><li key={f.name}>{f.name} · {bytes(f.received_bytes)} of {bytes(f.bytes)} received</li>)}</ul></details></>}{!item&&record?.phase==='staging'&&<><label>Original files<input type="file" multiple disabled={!!upload} onChange={e=>setFiles(Array.from(e.target.files||[]))}/></label><label>Original folder<input type="file" multiple {...{webkitdirectory:''}} disabled={!!upload} onChange={e=>setFiles(Array.from(e.target.files||[]))}/></label><button disabled={!files.length||!!upload} onClick={()=>void resume()}>Resume upload</button>{upload&&<UploadMeter value={upload}/>}</>}<p>Choose the entry to use when reviewing this import. Its destination changes only when you save a review.</p>{entry.collection_id&&<DestinationChooser collectionId={entry.collection_id} selected={item} onSelect={setItem}/>} {item&&<ImportWorkspace key={`${entry.target.id}:${item.id}`} initialImportId={entry.target.id} item={item} onPublished={onSaved} onError={onError}/>}</div>;
}

function WorkExecutions({entry,onError}:{entry:Entry;onError:(e:unknown)=>void}){
 type Row=components['schemas']['WorkJobView'];const [rows,setRows]=useState<Row[]>([]);const [cursor,setCursor]=useState<string|null>();const [changing,setChanging]=useState('');const [revision,setRevision]=useState(0);const pages=useRef(1);const refreshButton=useRef<HTMLButtonElement>(null);
 useEffect(()=>{const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;
 async function poll(){try{let after:string|null|undefined;const values:Row[]=[];for(let i=0;i<pages.current;i++){const page=await api<components['schemas']['Page_WorkJobView']>('/work/jobs',{method:'POST',body:JSON.stringify({target:entry.target,cursor:after||null,limit:25}),signal:controller.signal});values.push(...page.items);after=page.next_cursor;if(!after)break;}if(!controller.signal.aborted){setRows(values);setCursor(after);}}catch(e){if(!controller.signal.aborted)onError(e);}finally{if(!controller.signal.aborted)timer=setTimeout(poll,2000);}}
 void poll();return()=>{controller.abort();clearTimeout(timer);};},[entry.target.id,revision]);
 async function change(id:string,action:'retry'|'cancel'){setChanging(id);try{await api(`/jobs/${id}/${action}`,{method:'POST'});setRevision(v=>v+1);window.dispatchEvent(new Event('collector-jobs-updated'));}catch(e){onError(e);}finally{setChanging('');refreshButton.current?.focus();}}
 return <section aria-label="Task history"><div className="section-heading"><h4>Task history</h4><button ref={refreshButton} onClick={()=>setRevision(v=>v+1)}>Refresh tasks</button></div>{rows.length===0?<p>No tasks recorded.</p>:rows.map(({job,prior_failures})=><article key={job.id} aria-label={`${presentationLabel(job.kind)} ${presentationLabel(job.state)}`}><strong>{presentationLabel(job.kind)}</strong><p>{presentationLabel(job.state)} · {presentationLabel(job.phase)}</p>{job.provider_status&&<p role="status">{presentationLabel(job.provider_status)}</p>}<details><summary>Technical details</summary><p>Task ID: {job.id}</p><p>Kind: {job.kind} · Phase: {job.phase} · Attempt: {job.attempt}</p></details>{['queued','running','cancel_requested'].includes(job.state)&&<progress max={Math.max(1,job.total_units)} value={job.progress_kind==='indeterminate'?undefined:job.completed_units}/>}<p>{job.error||job.limitation}</p>{prior_failures.length>0&&<details><summary>Technical details: previous attempts ({prior_failures.length})</summary>{prior_failures.map(a=><p key={a.attempt}>Attempt {a.attempt}: {a.diagnostic} {a.finished_at&&new Date(a.finished_at).toLocaleString()}</p>)}</details>}{job.retryable&&!entry.archived&&!entry.clear_pending&&<button disabled={changing===job.id} onClick={()=>void change(job.id,'retry')}>Retry failed work</button>}{job.cancellable&&!entry.archived&&!entry.clear_pending&&<button disabled={changing===job.id} onClick={()=>void change(job.id,'cancel')}>Cancel task</button>}</article>)}{cursor&&<button onClick={()=>{pages.current++;setRevision(v=>v+1);}}>Load older tasks</button>}</section>;
}
