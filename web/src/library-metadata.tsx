import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage,retainUnchanged} from '@collector/runtime';
import {useBrowseScrollRestore,type BrowseRestore} from './browse-scroll-restore';
import type {components} from './generated/api';
import MetadataReview from './metadata-review';
import GuidedIngest from './guided-ingest';
import IngestStatus from './ingest-status';
import {presentationLabel} from './presentation-labels';
import {reasonLabels,reasonActions,reasonLevels,acceptable,type ReviewReason} from './review-reasons';
import BulkReview from './bulk-library-review';
import AlbumGrouping,{GroupUndo} from './album-grouping';
import {bytes} from './work-progress';
type S=components['schemas'];
type Item=S['PresentedItem'];
type Row=S['MetadataCareEntry'];
type Ingest=S['IngestSummary'];
type Counts=S['ReviewCounts'];
type Page<T>={items:T[];next_cursor:string|null};
export type ReviewTab='needs'|'progress'|'added'|'polish'|'all';
const tabs:ReviewTab[]=['needs','progress','added','polish','all'];
const entryTabs:ReviewTab[]=['needs','polish','all'];
function toRow(item:Item):Row{const art=item.assets.find(a=>a.id===item.primary_artwork);return {id:item.id,collection_id:item.collection_id,title:item.title,subtitle:item.subtitle,module:item.domain.module,artwork:art?{id:art.id,revision:art.revision,alt_text:art.alt_text}:null,summary_state:'ready',metadata_provisional:item.effective_metadata.title.source.source==='provisional',metadata_tentative:item.effective_metadata.resolution==='tentative',matching:null,reasons:[]};}
export type Props={restore:BrowseRestore;navigationRevision:number;userId:string;collection:string;search:string;itemId:string;ingestId:string;intake:string;tab:string;reason:string;batchId:string;groupIds:string;onBatch:(id:string)=>void;onLibrary:(item:Item)=>void;onOpenEntry:(item:string,collection:string)=>void;onOpenFormats:(item:string,collection:string)=>void;onQuery:(change:{reason?:string;tab?:string;intake?:string;item?:string;ingest?:string;group?:string},replace?:boolean)=>void;onChanged:(item:Item)=>void;onCounts?:(counts:Counts)=>void;onSessionExpired:()=>void};
function Cover({item}:{item:Item|Row}){const row='assets' in item?toRow(item):item;return row.artwork?<img loading="lazy" src={`/api/v1/collections/${row.collection_id}/assets/${row.artwork.id}/thumbnail?revision=${row.artwork.revision}`} alt={row.artwork.alt_text||`${row.title} cover`}/>:<span className="care-cover-empty" aria-label="No artwork">◇</span>;}

// Loads a cursor-paged list and keeps loading as the end of the list scrolls
// into view, so nothing hides behind a button. Polling refreshes the rows
// already shown without losing the reader's place.
function usePagedList<T>(key:string,path:(cursor:string|null)=>string,poll:number|((items:T[])=>number),onError:(error:unknown)=>void){
 const [items,setItems]=useState<T[]>([]),[cursor,setCursor]=useState<string|null>(null),[loading,setLoading]=useState(true),[pages,setPages]=useState(1),[version,setVersion]=useState(0);
 const fail=useRef(onError);fail.current=onError;
 const latestItems=useRef<T[]>([]);latestItems.current=items;
 const delay=()=>typeof poll==='function'?poll(latestItems.current):poll;
 useEffect(()=>{setItems([]);setCursor(null);setPages(1);},[key]);
 useEffect(()=>{const controller=new AbortController();let timer:ReturnType<typeof setTimeout>|undefined;
  // A catalog that changed or was busy mid-read is read again once at once,
  // rather than showing an error for a normal concurrent update.
  async function read(cursor:string|null,retried=false):Promise<Page<T>>{try{return await api<Page<T>>(path(cursor),{signal:controller.signal});}catch(error){if(!retried&&isApiRequestError(error)&&['revision_conflict','catalog_busy'].includes(error.code))return read(cursor,true);throw error;}}
  // Background refreshes keep the list as it is; only new reads show loading.
  async function load(background=false){if(!background)setLoading(true);try{const rows:T[]=[];let next:string|null=null;for(let n=0;n<pages;n++){const page:Page<T>=await read(next);rows.push(...page.items);next=page.next_cursor;if(!next)break;}
   if(controller.signal.aborted)return;latestItems.current=rows;setItems(current=>retainUnchanged(current,rows));setCursor(next);}
  catch(error){if(!controller.signal.aborted)fail.current(error);}
  finally{if(!controller.signal.aborted){setLoading(false);const wait=delay();if(wait)timer=setTimeout(()=>void load(true),wait);}}}
  void load();return()=>{controller.abort();if(timer)clearTimeout(timer);};
 },[key,pages,version]);
 return {items,more:!!cursor,loading,loadMore:()=>{if(cursor)setPages(value=>value+1);},reload:()=>setVersion(value=>value+1)};
}

function AutoMore({more,loading,onMore,shown,label}:{more:boolean;loading:boolean;onMore:()=>void;shown:number;label:string}){
 const sentinel=useRef<HTMLDivElement>(null);const latest=useRef(onMore);latest.current=onMore;
 useEffect(()=>{const element=sentinel.current;if(!element||!more||loading)return;const observer=new IntersectionObserver(entries=>{if(entries.some(entry=>entry.isIntersecting))latest.current();},{rootMargin:'400px'});observer.observe(element);return()=>observer.disconnect();},[more,loading]);
 return <div ref={sentinel} className="auto-more">{more?<button disabled={loading} onClick={onMore}>{loading?`Loading more ${label}…`:`Show more ${label}`}</button>:shown>0&&<small className="muted">All {shown} {label} shown</small>}</div>;
}

function ReasonChips({reasons}:{reasons:ReviewReason[]}){
 return <span className="reason-chips">{reasons.map(reason=><span key={reason} className={`reason-chip level-${reasonLevels[reason]}`}>{reasonLabels[reason]}</span>)}</span>;
}

function ingestState(summary:Ingest){
 if(summary.permissions.can_resume_upload)return 'Waiting for files';
 if(summary.phase==='staging')return 'Saving originals';
 if(summary.phase==='analyzing'||summary.phase==='publishing')return 'Saving originals';
 if(summary.phase==='awaiting_review')return 'Needs a choice';
 if(summary.outcome==='failed')return summary.capture_revision!=null?'Couldn’t finish · originals saved':'Couldn’t import';
 if(summary.phase==='published'&&summary.outcome==='active')return 'Identifying';
 return presentationLabel(summary.outcome);
}

function IngestRow({summary,selected,onOpen,action}:{summary:Ingest;selected:boolean;onOpen:()=>void;action?:React.ReactNode}){
 return <div className={`review-row ${selected?'selected':''}`}><button className="review-row-main" onClick={onOpen}><span className="care-cover-empty" aria-hidden="true">⇣</span><span className="row-text"><strong>{summary.package_name}</strong><small>{ingestState(summary)} · {summary.file_count} {summary.file_count===1?'file':'files'} · {bytes(summary.total_bytes)}{summary.capture_revision!=null?' · originals saved ✓':''}</small><IngestStatus summary={summary}/></span></button>{action}</div>;
}

// Finds the import an entry came from, so its steps can be shown in the panel.
function ImportDetails({item,collection,onChanged,onSessionExpired}:{item:Item;collection:string;onChanged:(item:Item)=>void;onSessionExpired:()=>void}){
 const [imports,setImports]=useState<string[]>([]),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{const controller=new AbortController();api<Page<S['WorkEntry']>>(`/work?item_id=${item.id}&limit=20`,{signal:controller.signal}).then(page=>setImports(page.items.flatMap(entry=>entry.target.kind==='import'?[entry.target.id]:[]))).catch(e=>{if(controller.signal.aborted)return;if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));});return()=>controller.abort();},[item.id]);
 if(!imports.length&&!error)return null;
 return <details id="fix-import" className="review-import-details"><summary>Import details ({imports.length})</summary>{error&&<p role="alert">{error}</p>}{imports.map(id=><GuidedIngest key={id} initialImportId={id} historyOnly collectionId={collection} busy={busy} setBusy={setBusy} onPublished={onChanged} onError={e=>setError(requestErrorMessage(e))}/>)}</details>;
}

export default function Review({restore,navigationRevision,userId,collection,search,itemId,ingestId,intake,tab:tabParam,reason,batchId,groupIds,onBatch,onLibrary,onOpenEntry,onOpenFormats,onQuery,onChanged,onCounts,onSessionExpired}:Props){
 const tab:ReviewTab=tabs.includes(tabParam as ReviewTab)?tabParam as ReviewTab:'needs';
 // Leaving the panel asks before dropping an unsaved edit or pausing an upload.
 function leave(action:()=>void){const panel=document.querySelector('.care-review');if(panel?.querySelector('[data-uploading="true"]')&&!window.confirm('Pause this upload and leave? To resume, select the same files again.'))return;if(!panel?.querySelector('[data-uploading="true"]')&&(panel?.matches('[data-unsaved="true"]')||panel?.querySelector('[data-unsaved="true"]'))&&!window.confirm('Discard unsaved changes and leave?'))return;action();}
 const [error,setError]=useState('');const [busy,setBusy]=useState(false);const [selected,setSelected]=useState<Item>();const [detailError,setDetailError]=useState('');
 const [counts,setCounts]=useState<Counts>();const [selection,setSelection]=useState<Row[]>([]);const [all,setAll]=useState<S['MetadataCareQuery']>();const [exclusions,setExclusions]=useState<string[]>([]);const [groupItems,setGroupItems]=useState<Item[]>([]);const [grouping,setGrouping]=useState(false);const [revision,setRevision]=useState(0);
 function fail(e:unknown){if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}
 const scope=new URLSearchParams();if(collection)scope.set('collection_id',collection);
 const entryQuery=(cursor:string|null)=>{const q=new URLSearchParams(scope);q.set('limit','50');if(search)q.set('search',search);if(reason)q.set('reason',reason);if(tab==='polish'){q.set('needs_attention','false');q.set('level','polish');}else q.set('needs_attention',String(tab==='needs'));if(cursor)q.set('cursor',cursor);return `/library-care/metadata?${q}`;};
 const ingestQuery=(status:'active'|'attention'|'added')=>(cursor:string|null)=>{const q=new URLSearchParams(scope);q.set('status',status);q.set('limit','50');if(intake)q.set('intake_id',intake);if(cursor)q.set('cursor',cursor);return `/ingests?${q}`;};
 const entryKey=JSON.stringify([userId,collection,search,reason,tab,revision]);
 // Rows still updating or being identified refresh quickly; settled lists slowly.
 const entries=usePagedList<Row>(entryTabs.includes(tab)?entryKey:'off',entryQuery,rows=>rows.some(row=>row.summary_state!=='ready'||row.matching&&['queued','running','cancel_requested'].includes(row.matching.state))?2000:10000,fail);
 useEffect(()=>{setAll(undefined);setExclusions([]);},[entryKey]);
 const problems=usePagedList<Ingest>(tab==='needs'?JSON.stringify([userId,collection,intake,'attention',revision]):'off',ingestQuery('attention'),5000,fail);
 const running=usePagedList<Ingest>(tab==='progress'?JSON.stringify([userId,collection,intake,'active',revision]):'off',ingestQuery('active'),2000,fail);
 const added=usePagedList<Ingest>(tab==='added'?JSON.stringify([userId,collection,intake,'added',revision]):'off',ingestQuery('added'),10000,fail);
 useEffect(()=>{const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;async function load(){try{const value=await api<Counts>(`/review/counts?${scope}`,{signal:controller.signal});if(controller.signal.aborted)return;setCounts(current=>retainUnchanged(current,value));onCounts?.(value);}catch(e){if(!controller.signal.aborted)fail(e);}if(!controller.signal.aborted)timer=setTimeout(()=>void load(),5000);}void load();return()=>{controller.abort();clearTimeout(timer);};},[userId,collection,revision]);
 useEffect(()=>{if(!groupIds){setGrouping(false);setGroupItems([]);return;}setGroupItems([]);const controller=new AbortController();const ids=groupIds.split(',');if(ids.length<2||ids.length>32){setError('Choose 2 to 32 albums to group.');return;}Promise.all(ids.map((id,index)=>api<Item>(`/items/${encodeURIComponent(id)}`,{signal:controller.signal,operation:`Load grouping entry ${index+1}`}))).then(items=>{if(!controller.signal.aborted){setSelection(items.map(toRow));setGroupItems(items);setGrouping(true);}}).catch(e=>{if(!controller.signal.aborted)fail(e);});return()=>controller.abort();},[groupIds]);
 useEffect(()=>{setSelected(undefined);setDetailError('');if(!itemId)return;const controller=new AbortController();
  api<Item>(`/items/${itemId}`,{signal:controller.signal}).then(item=>{if(collection&&item.collection_id!==collection){setDetailError('This entry is outside the selected collection.');return;}if(item.merged_into){onQuery({item:item.merged_into},true);return;}setSelected(item);}).catch(e=>{if(controller.signal.aborted)return;if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setDetailError(requestErrorMessage(e));});
  return()=>controller.abort();},[itemId,collection,revision]);
 useEffect(()=>{if(itemId||ingestId)document.querySelector<HTMLElement>('.care-review')?.focus({preventScroll:true});},[itemId,ingestId,selected?.id]);
 // Returning to the list restores its scroll position and focuses the row
 // that was open, through the app's shared navigation restore.
 useBrowseScrollRestore(restore,!itemId&&!ingestId&&!entries.loading&&entries.items.length>0,navigationRevision,true);
 async function previewBulk(){setBusy(true);setError('');try{const result=await api<S['BulkLibrarySummary']>('/bulk-library/preview',{method:'POST',body:JSON.stringify({kind:'metadata',selection:all?{selection:'all_matching',query:all,exclusions}:{selection:'explicit',items:selection.map(i=>i.id)},sources:{}})});onBatch(result.id);}catch(e){fail(e);}finally{setBusy(false);}}
 // Accepting needs each entry's current review state, so it is read first;
 // an entry that changed meanwhile keeps its reasons.
 async function acceptSelected(){setBusy(true);setError('');let accepted=0;try{for(const row of selection){const review=await api<S['MetadataReviewView']>(`/items/${row.id}/metadata`);let revisionNow=review.revision;for(const value of review.review_reasons.filter(acceptable)){const next=await api<S['MetadataReviewView']>(`/items/${row.id}/metadata`,{method:'PUT',body:JSON.stringify({expected_revision:revisionNow,change:{change:'accept_review',reason:value,state:review.review_state}})});revisionNow=next.revision;accepted++;}}setSelection([]);setRevision(v=>v+1);setError(accepted?'':'Nothing selected can be accepted as is; those reasons need a fix.');}catch(e){fail(e);}finally{setBusy(false);}}
 const visible=entryTabs.includes(tab)?entries.items:[];
 const position=visible.findIndex(row=>row.id===itemId);
 const next=position>=0?visible[position+1]:undefined;
 // Put focus back on the row that was open once the list shows again.
 function focusRow(id:string){const started=performance.now();const attempt=()=>{const row=document.querySelector<HTMLElement>(`.review-workspace [data-item-id="${CSS.escape(id)}"]`);if(row&&row.offsetParent){row.focus({preventScroll:true});if(document.activeElement===row)return;}if(performance.now()-started<1000)requestAnimationFrame(attempt);};requestAnimationFrame(attempt);}
 const openEntry=(row:Row)=>leave(()=>{setGrouping(false);onQuery({item:row.id,ingest:''});});
 const label=(value:ReviewTab)=>value==='needs'?`Needs you${counts?` ${counts.needs_you}`:''}`:value==='progress'?'In progress':value==='added'?'Added recently':value==='all'?'All entries':`Polish${counts?` ${counts.polish}`:''}`;
 const panelOpen=!!itemId||!!ingestId||grouping||!!batchId;
 return <section className={`care-workspace review-workspace ${panelOpen?'care-review-open':''}`} aria-label="Review workspace"><nav className="review-tabs" aria-label="Review lists">{tabs.map(value=><button key={value} aria-current={tab===value?'page':undefined} onClick={()=>leave(()=>onQuery({tab:value,item:'',ingest:''}))}>{label(value)}</button>)}</nav><div className="care-list">
  
  {!!counts?.setup&&<p className="notice review-setup">{counts.setup} {counts.setup===1?'entry is':'entries are'} waiting for a game catalog for their system. They are checked again automatically once a catalog is activated in Administration → Game catalogs.</p>}
  {intake&&<p className="notice">Showing one import only. <button onClick={()=>onQuery({intake:''})}>Show all imports</button></p>}
  {entryTabs.includes(tab)&&<div className="care-filters"><label>Reason<select aria-label="Reason" value={reason} onChange={event=>onQuery({reason:event.target.value,item:''})}><option value="">Any reason</option>{(Object.keys(reasonLabels) as ReviewReason[]).filter(value=>tab==='all'||(tab==='polish'?reasonLevels[value]==='polish':reasonLevels[value]!=='polish'&&reasonLevels[value]!=='setup')).map(value=><option value={value} key={value}>{reasonLabels[value]}</option>)}</select></label></div>}
  {error&&<p role="alert" className="notice error">{error}</p>}
  {tab==='needs'&&(problems.items.length>0||problems.loading&&!problems.items.length)&&<section className="review-group" aria-label="Imports that need you"><h2>Imports that need you</h2>{problems.items.map(summary=><IngestRow key={summary.import_id} summary={summary} selected={ingestId===summary.import_id} onOpen={()=>leave(()=>onQuery({ingest:summary.import_id,item:''}))}/>)}<AutoMore more={problems.more} loading={problems.loading} onMore={problems.loadMore} shown={problems.items.length} label="imports"/></section>}
  {entryTabs.includes(tab)&&<section className="review-group" aria-label={tab==='needs'?'Entries that need you':tab==='all'?'All entries':'Entries that could be better'}>{tab==='needs'&&<h2>Entries that need you</h2>}
   {(!!selection.length||all)&&<div className="care-selection" role="region" aria-label="Selected entries"><strong>{all?'All matching entries':selection.length+' selected'}{all&&exclusions.length?` except ${exclusions.length}`:''}</strong>{!all&&<button disabled={busy} onClick={()=>void acceptSelected()}>Accept as is</button>}<button disabled={busy} onClick={()=>void previewBulk()}>Look up again</button>{!all&&selection.length>0&&selection.every(item=>item.module==='org.junk.music')&&<button disabled={selection.length<2||new Set(selection.map(item=>item.collection_id)).size!==1} onClick={()=>{setGrouping(true);onQuery({item:'',group:selection.map(i=>i.id).join(',')});}}>Group into one album</button>}<button onClick={()=>{setSelection([]);setAll(undefined);setExclusions([]);setGrouping(false);}}>Clear selection</button></div>}
   {entries.items.length>0&&!all&&<button className="review-select-all" onClick={()=>{setAll({collection_id:collection||null,search,reason:(reason||null) as S['MetadataCareQuery']['reason'],level:tab==='polish'?'polish':null,needs_attention:tab==='needs',cursor:null,limit:null});setExclusions([]);setSelection([]);}}>Select all matching entries</button>}
   <div className="care-rows" aria-busy={entries.loading}>{entries.items.map(row=><div className={`review-row ${itemId===row.id?'selected':''}`} key={row.id}><label className="review-select"><input type="checkbox" aria-label={`Select ${row.title}`} checked={all?!exclusions.includes(row.id):selection.some(value=>value.id===row.id)} onChange={event=>{if(all)setExclusions(current=>event.target.checked?current.filter(id=>id!==row.id):[...current,row.id]);else setSelection(current=>event.target.checked?[...current,row]:current.filter(value=>value.id!==row.id));}}/></label><button className="review-row-main care-row" data-item-id={row.id} onClick={()=>openEntry(row)}><Cover item={row}/><span className="row-text"><strong>{row.title}</strong><small>{row.subtitle||(row.module==='org.junk.games'?'No system or edition details':'No artist or edition details')}</small>{row.matching&&['queued','running','cancel_requested'].includes(row.matching.state)?<small>{row.matching.phase==='queued'?'Queued':'Identifying'}</small>:row.summary_state!=='ready'?<small>Updating</small>:<ReasonChips reasons={row.reasons}/>}</span></button>{row.reasons[0]&&<button className="review-action" onClick={()=>openEntry(row)}>{reasonActions[row.reasons[0]]}</button>}</div>)}</div>
   {!entries.items.length&&!entries.loading&&<p className="review-empty">{tab==='needs'?'Nothing needs you right now.':tab==='all'?'No entries match.':'Nothing to polish.'}</p>}
   <AutoMore more={entries.more} loading={entries.loading} onMore={entries.loadMore} shown={entries.items.length} label="entries"/>
  </section>}
  {tab==='progress'&&<section className="review-group" aria-label="Imports in progress">{running.items.map(summary=><IngestRow key={summary.import_id} summary={summary} selected={ingestId===summary.import_id} onOpen={()=>leave(()=>onQuery({ingest:summary.import_id,item:''}))}/>)}{!running.items.length&&!running.loading&&<p className="review-empty">Nothing is importing right now.</p>}<AutoMore more={running.more} loading={running.loading} onMore={running.loadMore} shown={running.items.length} label="imports"/></section>}
  {tab==='added'&&<section className="review-group" aria-label="Added recently">{added.items.map(summary=><IngestRow key={summary.import_id} summary={summary} selected={ingestId===summary.import_id} onOpen={()=>leave(()=>onQuery({ingest:summary.import_id,item:''}))} action={summary.target_item_id&&<button className="review-action" onClick={()=>onOpenEntry(summary.target_item_id!,summary.collection_id)}>Open in Library</button>}/>)}{!added.items.length&&!added.loading&&<p className="review-empty">Nothing has been added yet.</p>}<AutoMore more={added.more} loading={added.loading} onMore={added.loadMore} shown={added.items.length} label="imports"/></section>}
 </div>
 {batchId&&<div><button className="care-return" onClick={()=>onBatch('')}>Back to the list</button><BulkReview key={batchId} id={batchId} onOpen={(item)=>{onBatch('');onQuery({item});}} onSessionExpired={onSessionExpired}/></div>}
 {!batchId&&grouping&&!!groupItems.length&&<AlbumGrouping key={groupIds} userId={userId} items={groupItems} onCancel={()=>{setGrouping(false);onQuery({group:''});}} onCompleted={destination=>{setGrouping(false);setSelection([]);setRevision(value=>value+1);onQuery({item:destination,group:''});}}/>}
 {ingestId&&!itemId&&!grouping&&!batchId&&<section className="care-review" aria-label="Selected import" tabIndex={-1}><button className="care-return" onClick={()=>leave(()=>onQuery({ingest:''}))}>Back to the list</button><GuidedIngest key={ingestId} initialImportId={ingestId} historyOnly collectionId={collection} busy={busy} setBusy={setBusy} onPublished={item=>{onChanged(item);setRevision(v=>v+1);}} onError={fail}/></section>}
 {itemId&&!grouping&&!batchId&&<section className="care-review" aria-label="Review selected entry" tabIndex={-1}><div className="review-panel-nav"><button className="care-return" onClick={()=>leave(()=>{const open=itemId;onQuery({item:''});focusRow(open);})}>Back to the list</button>{next&&<button className="primary" onClick={()=>openEntry(next)}>Next entry ›</button>}</div>{detailError&&<p role="alert" className="notice error">{detailError}</p>}{selected?<><header><Cover item={selected}/><div><h2>{selected.title}</h2><p>{selected.subtitle}</p><p className="muted">{selected.captures.length?'Original files saved ✓':'No original files attached'}</p><button onClick={()=>onLibrary(selected)}>Open in Library</button></div></header>{selected.merge_group&&<GroupUndo item={selected} groupId={selected.merge_group} onChanged={()=>{setRevision(value=>value+1);onQuery({item:''});}}/>}{selected.domain.module==='org.junk.music'&&<RelatedAlbums key={`groups:${selected.id}`} item={selected} onReview={items=>{setSelection(items.map(toRow));setGroupItems(items);setGrouping(true);onQuery({item:'',group:items.map(i=>i.id).join(',')});}} onSessionExpired={onSessionExpired}/>}<MetadataReview key={`care:${selected.id}:${selected.merge_group||'ungrouped'}`} userId={userId} item={selected} onChanged={item=>{setSelected(item);onChanged(item);entries.reload();}} onSessionExpired={onSessionExpired}/>{selected.domain.module==='org.junk.music'&&<section id="fix-formats" className="review-formats" aria-label="Saved formats"><h4>Saved formats</h4><p className="muted">The originals always play. Extra FLAC or Opus copies, and any that were only partly made, are managed in Saved formats.</p><button onClick={()=>onOpenFormats(selected.id,selected.collection_id)}>Open in Saved formats</button></section>}<ImportDetails item={selected} collection={selected.collection_id} onChanged={onChanged} onSessionExpired={onSessionExpired}/>{next&&<div className="review-panel-nav"><button className="primary" onClick={()=>openEntry(next)}>Next entry ›</button></div>}</>:!detailError&&<p role="status">Loading…</p>}</section>}
 </section>;
}

function GroupingSuggestions({item,onReview,onSessionExpired}:{item:Item;onReview:(items:Item[])=>void;onSessionExpired:()=>void}){const [items,setItems]=useState<Item[]>([]),[selected,setSelected]=useState<string[]>([]),[cursor,setCursor]=useState<string|null>(null),[error,setError]=useState('');useEffect(()=>{const controller=new AbortController();api<{items:Item[];next_cursor:string|null}>(`/items/${item.id}/album-group-proposals?limit=25`,{signal:controller.signal}).then(p=>{setItems(p.items);setCursor(p.next_cursor);}).catch(e=>{if(controller.signal.aborted)return;if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));});return()=>controller.abort();},[item.id,item.revision]);return error?<p role="alert">{error}</p>:items.length?<section aria-label="Album grouping proposals"><h3>Possibly the same album</h3>{items.map(i=><label className="care-row" key={i.id}><input type="checkbox" checked={selected.includes(i.id)} onChange={e=>setSelected(v=>e.target.checked?[...v,i.id]:v.filter(id=>id!==i.id))}/><Cover item={i}/><strong>{i.title}</strong></label>)}{cursor&&<button onClick={async()=>{try{const p=await api<{items:Item[];next_cursor:string|null}>(`/items/${item.id}/album-group-proposals?limit=25&cursor=${cursor}`);setItems(v=>[...v,...p.items]);setCursor(p.next_cursor);}catch(e){setError(requestErrorMessage(e));}}}>More proposals</button>}<button disabled={!selected.length||selected.length>31} onClick={()=>onReview([item,...items.filter(i=>selected.includes(i.id))])}>Review selected grouping</button></section>:null;}

function RelatedAlbums(props:React.ComponentProps<typeof GroupingSuggestions>){const [open,setOpen]=useState(false);return <details id="fix-group" onToggle={event=>setOpen(event.currentTarget.open)}><summary>Related albums</summary>{open&&<GroupingSuggestions {...props}/>}</details>;}
