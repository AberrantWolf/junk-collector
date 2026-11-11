import ImportFileTree from './import-file-tree';
import MoreButton from './more-button';
import ImportAlbumEditor,{type AlbumTarget} from './import-album-editor';
import {ImportPreparation,importPreparation,type ImportPreparationChoice} from './audio-profiles';
import PurchaseFields from './purchase-fields';
import IngestStatus from './ingest-status';
import {presentationLabel} from './presentation-labels';
import React,{useEffect,useRef,useState,api,requestErrorMessage,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';
import InboxPackageChooser from './inbox-package-chooser';
import DestinationChooser from './destination-chooser';
import GuidedIngest from './guided-ingest';
import {useDialogDismiss} from './dialog';
import {uploadPackage,type UploadProgress} from './package-upload';
import {loadDraft,saveDraft} from './intake-drafts';
import {UploadMeter,bytes} from './work-progress';
type PendingManifest={key:string;id?:string;packages:Package[];automatic:boolean;prepare:boolean;audio_preparation?:S['ImportAudioPreparation'];reviewed_opus_import?:S['OpusImportDefault']};
type S=components['schemas'];type Session=S['IntakeSession'];type Member=S['IntakeMemberView'];type Plan=S['IngestPlanPackage'];type Package=S['IntakePackage'];type Item=S['PresentedItem'];
export type Props={userId:string;collection:string;itemId:string;collections:S['PresentedCollection'][];onQuery:(v:{item?:string;collection?:string})=>void;onMetadata:(item:string,collection:string)=>void;onChanged:()=>void;onSessionExpired:()=>void};
type FolderLayout='auto'|'items'|'album';
const transfers=new Map<string,{controller:AbortController;progress?:UploadProgress;error?:string;running:boolean}>();
const originalFiles=new Map<string,File[]>();
window.addEventListener('collector-api-identity-changed',event=>{const user=(event as CustomEvent<{userId?:string}>).detail.userId;for(const [key,task] of transfers)if(!user||!key.startsWith(user+':'))task.controller.abort();});
const name=(f:File)=>f.webkitRelativePath||f.name;
const digitalPackage=(p:Package)=>p.package.files.some(f=>/\.(flac|mp3|wav)$/i.test(f.name))&&!p.package.files.some(f=>/\.(cue|toc)$/i.test(f.name));
async function transferBatch(user:string,id:string,files:File[]){const key=user+':'+id;if(transfers.get(key)?.running)return;const task={controller:new AbortController(),running:true,progress:undefined as UploadProgress|undefined,error:undefined as string|undefined};transfers.set(key,task);const signal=task.controller.signal;
 try{let cursor:string|null=null;let lastPosition=-1;
  for(;;){signal.throwIfAborted();const page:{items:Member[];next_cursor:string|null}=await api<{items:Member[];next_cursor:string|null}>(`/intakes/${id}/members?limit=32${cursor?'&cursor='+cursor:''}`,{signal});
   let waiting=false;for(const member of page.items){if(!member.import_id&&!member.error){waiting=true;break;}lastPosition=member.position;if(!member.import_id||member.plan.source.kind!=='browser')continue;const detail=await api<S['IngestDetail']>(`/ingests/${member.import_id}`,{signal});if(detail.summary.permissions.can_resume_upload){const names=new Set(detail.import.files.map(f=>f.name));await uploadPackage(detail.import,files.filter(f=>names.has(name(f))),true,signal,p=>{task.progress=p;},()=>{});task.progress=undefined;}}
   if(!waiting&&page.next_cursor){cursor=page.next_cursor;continue;}const session=await api<Session>(`/intakes/${id}`,{signal});if(!waiting&&lastPosition+1>=session.package_count)break;if(['cancelled','cancelling'].includes(session.state))break;
   cursor=lastPosition>=0?String(lastPosition):null;await new Promise(resolve=>setTimeout(resolve,1000));
  }
 }catch(e){if(!signal.aborted)task.error=requestErrorMessage(e);}finally{task.running=false;task.progress=undefined;}
}
type IntakePage<T>={items:T[];next_cursor:string|null};
function SavedImports({collection,onOpen,onSessionExpired}:{collection:string;onOpen:(session:Session)=>void;onSessionExpired:()=>void}){
 const [page,setPage]=useState<IntakePage<Session>>();
 const [loading,setLoading]=useState(true);
 const [error,setError]=useState('');
 const request=useRef<AbortController|null>(null);

 async function load(cursor?:string){
  if(request.current)return;
  const controller=new AbortController();request.current=controller;
  const {signal}=controller;
  const params=new URLSearchParams({limit:'50'});
  if(collection)params.set('collection_id',collection);
  if(cursor)params.set('cursor',cursor);
  setLoading(true);setError('');
  try{
   const next=await api<IntakePage<Session>>(`/intakes?${params}`,{signal});
   setPage(previous=>({items:cursor?[...(previous?.items||[]),...next.items]:next.items,next_cursor:next.next_cursor}));
  }catch(error){
   if(signal.aborted)return;
   if(isApiRequestError(error)&&error.code==='unauthenticated')onSessionExpired();
   else setError(requestErrorMessage(error));
  }finally{
   if(request.current===controller)request.current=null;
   if(!signal.aborted)setLoading(false);
  }
 }
 useEffect(()=>{
  void load();
  return()=>{request.current?.abort();request.current=null;};
 },[]);
 return <section aria-label="Saved imports">
  <h2>Saved imports</h2>
  <div className="striped-imports" aria-busy={loading}>{page?.items.map(session=><button className="collection-row" key={session.id} onClick={()=>onOpen(session)}>
   <span className="row-text"><strong>{new Date(session.created_at).toLocaleString()} · {session.package_count} packages</strong>
   <small>{presentationLabel(session.state)} · {session.preserved} originals saved · {session.needs_attention} need attention</small></span>
  </button>)}</div>
  {error&&<p role="alert" className="notice error">{error} {!page&&<button onClick={()=>void load()}>Retry loading imports</button>}</p>}
  {!page&&loading&&<p role="status">Loading saved imports…</p>}
  {page?.next_cursor&&<MoreButton loading={loading} loadingText="Loading imports…" onClick={()=>void load(page.next_cursor!)}>More imports</MoreButton>}
 </section>;
}

function albumTarget(member:Member):AlbumTarget|undefined{
 const item=member.import;
 if(member.plan.music_actions_available&&item?.capture_revision!=null&&item.target_item_id){
  return {id:item.target_item_id,title:member.plan.default_title};
 }
}

function ImportedMembers({id,members,cursor,loading,busy,onMore,onChanged,onSessionExpired,children}:{
 id:string;members:Member[];cursor:string|null;loading:boolean;busy:boolean;onMore:()=>void;onChanged:()=>void;onSessionExpired:()=>void;
 children:(member:Member)=>React.ReactNode;
}){
 const [selection,setSelection]=useState(new Set<string>());
 const [all,setAll]=useState(false);
 const [editor,setEditor]=useState<AlbumTarget[]>();
 const [preparing,setPreparing]=useState(false);
 const [error,setError]=useState('');
 const selectAll=useRef<HTMLInputElement>(null);
 const editButton=useRef<HTMLButtonElement>(null);
 const request=useRef<AbortController|null>(null);
 useEffect(()=>()=>request.current?.abort(),[]);
 const selected=(id:string)=>all!==selection.has(id);
 const selectable=members.some(member=>albumTarget(member));
 const hasSelection=members.some(member=>{const album=albumTarget(member);return album&&selected(album.id);});
 const locked=busy||preparing||!!editor;
 useEffect(()=>{if(selectAll.current)selectAll.current.indeterminate=selection.size>0;},[selection,selectable]);

 async function openEditor(){
  if(request.current)return;
  const controller=new AbortController();request.current=controller;
  const {signal}=controller;
  const albums=new Map<string,AlbumTarget>();
  const add=(rows:Member[])=>{
   for(const member of rows){
    const album=albumTarget(member);
    if(album&&selected(album.id)&&!albums.has(album.id))albums.set(album.id,album);
   }
  };
  setPreparing(true);setError('');
  try{
   add(members);
   let next=all?cursor:null;
   while(next){
    const page:IntakePage<Member>=await api(`/intakes/${id}/members?limit=32&cursor=${next}`,{signal});
    add(page.items);next=page.next_cursor;
   }
   if(albums.size)setEditor([...albums.values()]);
   else setError('No published albums are selected.');
  }catch(error){
   if(signal.aborted)return;
   if(isApiRequestError(error)&&error.code==='unauthenticated')onSessionExpired();
   else setError(requestErrorMessage(error));
  }finally{
   request.current=null;
   if(!signal.aborted)setPreparing(false);
  }
 }

 return <>
  {selectable&&<div className="intake-selection">
   <label className="intake-select"><input ref={selectAll} type="checkbox" checked={all&&!selection.size} disabled={locked} onChange={()=>{
    setAll(!all||selection.size>0);setSelection(new Set());
   }}/> Select all imported albums</label>
   <button ref={editButton} disabled={locked||(!hasSelection&&!(all&&cursor))} onClick={()=>void openEditor()}>Edit selected</button>
   {preparing&&<span role="status">Finding selected albums…</span>}
  </div>}
  {error&&<p role="alert" className="notice error">{error}</p>}
  <div className="intake-members striped-imports" aria-busy={loading}>{members.map(member=>{
   const album=albumTarget(member);
   return <div className="care-row" key={member.position}>
    {album&&<label className="intake-select"><input type="checkbox" aria-label={`Select ${album.title}`} checked={selected(album.id)} disabled={locked}
     onChange={event=>{const checked=event.target.checked;setSelection(current=>{
      const next=new Set(current);if(checked!==all)next.add(album.id);else next.delete(album.id);return next;
     });}}/></label>}
    {children(member)}
   </div>;
  })}</div>
  {cursor&&<MoreButton loading={loading} loadingText="Loading packages…" disabled={locked} onClick={onMore}>More packages</MoreButton>}
  {editor&&<ImportAlbumEditor albums={editor} onClose={()=>{setEditor(undefined);requestAnimationFrame(()=>editButton.current?.focus());}} onChanged={onChanged} onSessionExpired={onSessionExpired}/>}
 </>;
}

export default function Imports({userId,collection,itemId,collections,onQuery,onMetadata,onChanged,onSessionExpired}:Props){
 const [session,setSession]=useState<Session>(),[admissionJob,setAdmissionJob]=useState<S['JobView']>(),[members,setMembers]=useState<Member[]>([]),[cursor,setCursor]=useState<string|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false),[tick,setTick]=useState(0),[files,setFiles]=useState<File[]>([]),[packages,setPackages]=useState<Package[]>([]),[plans,setPlans]=useState<Plan[]>([]),[unassigned,setUnassigned]=useState<string[]>([]),[unassignedPage,setUnassignedPage]=useState(0),[automatic,setAutomatic]=useState(true),[audioChoice,setAudioChoice]=useState<ImportPreparationChoice>('default'),[opusReview,setOpusReview]=useState<{collection:string;userId:string;plans:Plan[];settings:S['OpusImportDefault']}>(),[source,setSource]=useState<'browser'|'inbox'>('browser'),[inboxes,setInboxes]=useState<S['InboxSourceView'][]>([]),[label,setLabel]=useState(''),[selectedInbox,setSelectedInbox]=useState<string[]>([]),[reviewPage,setReviewPage]=useState(0),[pageCount,setPageCount]=useState(1),[hydrated,setHydrated]=useState(''),[pending,setPending]=useState<PendingManifest>(),[detail,setDetail]=useState(''),[destination,setDestination]=useState<Item|null>(null),[transfer,setTransfer]=useState<{running:boolean;progress?:UploadProgress;error?:string}>();
 const [folderLayout,setFolderLayout]=useState<FolderLayout>('auto');
 const [reviewVersion,setReviewVersion]=useState(0);
 const [loadingMembers,setLoadingMembers]=useState(false),[loadedPages,setLoadedPages]=useState(0);
 const reviewDialog=useDialogDismiss(()=>setDetail(''),!!detail);
 const musicCollection=plans.length>0?plans.every(plan=>plan.music_actions_available):collections.some(c=>c.id===collection&&c.binding.state==='assigned'&&c.binding.module==='org.junk.music');
 const scope=useRef(new AbortController());const draftKey=`collector:intake-draft:${userId}:${collection}`;
 const pendingKey=draftKey+':manifest';
 const opusDefault=opusReview?.collection===collection&&opusReview.userId===userId&&opusReview.plans===plans?opusReview.settings:undefined;
 useEffect(()=>{
  setOpusReview(undefined);
  if(!collection||!musicCollection||!plans.length)return;
  const controller=new AbortController();
  void api<S['CdLibrarySettings']>(`/collections/${collection}/cd-settings`,{signal:controller.signal}).then(settings=>{
   if(!controller.signal.aborted)setOpusReview({collection,userId,plans,settings:settings.opus_import??{scope:'off',bitrate_kbps:128}});
  }).catch(e=>{if(!controller.signal.aborted)fail(e);});
  return()=>controller.abort();
 },[collection,musicCollection,userId,plans]);
 const sourceBusy=busy||hydrated!==draftKey;
 const reviewLocked=sourceBusy||!!pending;
 function fail(e:unknown){if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}
 async function mutate(action:()=>Promise<void>){const signal=scope.current.signal;setBusy(true);setError('');try{await action();}catch(e){if(!signal.aborted)fail(e);}finally{if(!signal.aborted)setBusy(false);}}
 useEffect(()=>{scope.current=new AbortController();setSession(undefined);setBusy(false);setAdmissionJob(undefined);setMembers([]);setError('');setHydrated('');setFolderLayout('auto');setPackages([]);setPlans([]);setPending(undefined);setFiles(originalFiles.get(draftKey)||[]);
 const draftSignal=scope.current.signal;void Promise.all([loadDraft<{packages:Package[];plans:Plan[];unassigned:string[];source:'browser'|'inbox';automatic:boolean;prepare?:boolean;audioChoice?:ImportPreparationChoice;folderLayout?:FolderLayout}>(draftKey),loadDraft<PendingManifest>(pendingKey)]).then(([d,p])=>{if(draftSignal.aborted)return;setPackages(d?.packages||[]);setPlans(d?.plans||[]);setUnassigned(d?.unassigned||[]);setSource(d?.source||'browser');setFolderLayout(d?.folderLayout??'auto');setAutomatic(d?.automatic??true);setAudioChoice(d?.audioChoice??(d?.prepare?'flac':'default'));setPending(p);setHydrated(draftKey);}).catch(e=>{if(!draftSignal.aborted)fail(e);});
 const signal=scope.current.signal;if(collection)api<S['InboxSourceView'][]>(`/collections/${collection}/inboxes?packages=false`,{signal}).then(p=>{if(!signal.aborted){setInboxes(p);setLabel(p[0]?.label||'');}}).catch(e=>{if(!signal.aborted)fail(e);});return()=>scope.current.abort();},[userId,collection]);
 const draft=React.useMemo(()=>({packages,plans,unassigned,source,automatic,audioChoice,folderLayout}),[packages,plans,unassigned,source,automatic,audioChoice,folderLayout]);
 const [savedDraft,setSavedDraft]=useState<typeof draft>();
 useEffect(()=>{
  if(hydrated!==draftKey||!draft.packages.length)return;
  let active=true;
  void saveDraft(draftKey,draft).then(()=>{if(active)setSavedDraft(draft);}).catch(error=>{if(active)fail(error);});
  return()=>{active=false;};
 },[draft,draftKey,hydrated]);
 useEffect(()=>{setPageCount(1);setLoadedPages(0);setLoadingMembers(false);setMembers([]);setCursor(null);setDetail('');},[itemId]);
 useEffect(()=>{if(!itemId)return;const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;async function poll(){try{const s=await api<Session>(`/intakes/${itemId}`,{signal:controller.signal});const job=s.state==='draft'?undefined:await api<S['JobView']>(`/jobs/${itemId}`,{signal:controller.signal});let next:string|null=null;const loaded:Member[]=[];for(let n=0;n<pageCount;n++){const p:{items:Member[];next_cursor:string|null}=await api(`/intakes/${itemId}/members?limit=32${next?'&cursor='+next:''}`,{signal:controller.signal});loaded.push(...p.items);next=p.next_cursor;if(!next)break;}if(!controller.signal.aborted){setSession(s);setAdmissionJob(job);setMembers(loaded);setCursor(next);setLoadedPages(pageCount);const task=transfers.get(userId+':'+itemId);setTransfer(task?{...task}:undefined);}}catch(e){if(!controller.signal.aborted)fail(e);}finally{if(!controller.signal.aborted)setLoadingMembers(false);}if(!controller.signal.aborted)timer=setTimeout(poll,2000);}void poll();return()=>{controller.abort();clearTimeout(timer);};},[itemId,userId,collection,tick,pageCount]);
 useEffect(()=>{if(!transfer?.running)return;const warn=(e:BeforeUnloadEvent)=>{e.preventDefault();e.returnValue='';};window.addEventListener('beforeunload',warn);return()=>window.removeEventListener('beforeunload',warn);},[transfer?.running]);
 const reviewedPackages=pending?.packages??packages;
 const needsReviewedFiles=reviewedPackages.some(entry=>entry.package.source.kind==='browser')&&!files.length;
 const reviewedFolder=needsReviewedFiles&&reviewedPackages.some(entry=>entry.package.files.some(file=>file.name.includes('/')));

 function reselectReviewedFiles(selected:File[]){
  const inventory=new Map(selected.map(file=>[name(file),file.size]));
  if(inventory.size!==selected.length||reviewedPackages.some(entry=>entry.package.files.some(file=>inventory.get(file.name)!==file.bytes))){
   setError('The selected files differ from this review. Reselect the original files or start a new review.');
   return;
  }
  setError('');
  setFiles(selected);
  originalFiles.set(draftKey,selected);
 }

 function replaceReview(found:Plan[],unused:string[],selected:File[]){
  setReviewVersion(version=>version+1);
  setPlans(found);
  setPackages(found.map(plan=>({
   package:{
    album_tracks:plan.album_tracks,
    source:plan.source,
    files:plan.files,
    anchor_file_index:plan.anchors.find(anchor=>anchor.selected)?.file_index??null,
   },
   destination:{destination:'separate'},
  })));
  setUnassigned(unused);
  setUnassignedPage(0);
  setReviewPage(0);
  setFiles(selected);
  originalFiles.set(draftKey,selected);
 }

 async function chooseFiles(selected:File[],folder:boolean,layout:FolderLayout=folderLayout){
  if(!selected.length)return;
  const signal=scope.current.signal;
  await mutate(async()=>{
   if(selected.length>100000)throw new Error('Choose at most 100,000 files per intake. Split the remaining folders into another intake.');
   const input=selected.map(file=>({name:name(file),bytes:file.size,sha256:null}));
   const found:Plan[]=[];
   const unused:string[]=[];
   if(folder){
    const roots=new Set(selected.map(file=>file.webkitRelativePath.split('/')[0]));
    const [selectedDirectory]=roots;
    const albumRoot=layout==='album'||layout==='auto'&&selected.some(file=>{
     if(!/\.(flac|wav|mp3)$/i.test(file.name))return false;
     const parts=file.webkitRelativePath.split('/');
     return parts.length===2||/^(?:flac|wav|mp3|(?:disc|cd)[ _-]*[0-9]+)$/i.test(parts[1]||'');
    });
    const discovery=await api<S['IntakeDiscovery']>('/intake-discoveries',{
     method:'POST',signal,
     body:JSON.stringify({collection_id:collection,directory:musicCollection&&roots.size===1&&!albumRoot?selectedDirectory:null,files:input}),
    });
    let cursor:string|null=null,unusedCursor:string|null=null;
    let packagesDone=false,unusedDone=false;
    do{
     const params=new URLSearchParams();
     if(cursor)params.set('cursor',cursor);
     if(unusedCursor)params.set('unassigned_cursor',unusedCursor);
     const page:S['IntakeDiscoveryPage']=await api(`/intake-discoveries/${discovery.id}?${params}`,{signal});
     if(!packagesDone)found.push(...page.packages.items);
     if(!unusedDone)unused.push(...page.unassigned_files.items);
     cursor=page.packages.next_cursor??null;
     unusedCursor=page.unassigned_files.next_cursor??null;
     packagesDone||=!cursor;
     unusedDone||=!unusedCursor;
    }while(!packagesDone||!unusedDone);
   }else{
    const plan=await api<S['IngestPlan']>('/ingests/plan',{
     method:'POST',signal,
     body:JSON.stringify({collection_id:collection,packages:[{source:{kind:'browser',label:null,package_name:selected[0].name},files:input,anchor_file_index:null}]}),
    });
    found.push(...plan.packages);
   }
   signal.throwIfAborted();
   replaceReview(found,unused,selected);
   setFolderLayout(layout);
  });
 }

 async function previewInbox(){
  const signal=scope.current.signal;
  await mutate(async()=>{
   const found:Plan[]=[];
   for(let offset=0;offset<selectedInbox.length;offset+=64){
    const packages=selectedInbox.slice(offset,offset+64).map(package_name=>({source:{kind:'inbox',label,package_name},files:[],anchor_file_index:null}));
    const plan=await api<S['IngestPlan']>('/ingests/plan',{method:'POST',signal,body:JSON.stringify({collection_id:collection,packages})});
    found.push(...plan.packages);
   }
   signal.throwIfAborted();
   replaceReview(found,[],[]);
  });
 }
 async function chooseAnchor(position:number,fileIndex:number|null){const signal=scope.current.signal;const selected=packages[position];await mutate(async()=>{
  const value=await api<S['IngestPlan']>('/ingests/plan',{method:'POST',signal,body:JSON.stringify({collection_id:collection,packages:[{...selected.package,anchor_file_index:fileIndex}]})});
  signal.throwIfAborted();const plan=value.packages[0];
  setPlans(current=>current.map((p,i)=>i===position?plan:p));
  setPackages(current=>current.map((p,i)=>i===position?{...p,package:{...p.package,files:plan.files,anchor_file_index:plan.anchors.find(a=>a.selected)?.file_index??null}}:p));
 });}
 function groupTitle(key:string,title:string){setPackages(current=>current.map(p=>p.destination.destination==='album_group'&&p.destination.key===key?{...p,destination:{...p.destination,title:title.trim()?title:null}}:p));}
 const missingAlbumSelection=packages.some((entry,index)=>entry.package.album_tracks?!entry.package.album_tracks.length:plans[index]?.music_layout?.needs_attention);
 const missingCopy=packages.some(p=>p.destination.destination!=='separate'&&!p.destination.copy_key.trim());
 async function start(){const signal=scope.current.signal;await mutate(async()=>{
  let intent=pending;if(!intent){intent={key:crypto.randomUUID(),packages,automatic,prepare:audioChoice==='flac'&&musicCollection,audio_preparation:musicCollection?importPreparation(audioChoice):undefined,reviewed_opus_import:musicCollection&&audioChoice==='default'?opusDefault:undefined};await saveDraft(pendingKey,intent);signal.throwIfAborted();setPending(intent);}
  let saved:Session=intent.id?await api<Session>(`/intakes/${intent.id}`,{signal}):await api<Session>('/intakes',{method:'POST',signal,headers:{'Idempotency-Key':intent.key},body:JSON.stringify({collection_id:collection,automatic_metadata:intent.automatic,prepare_music:intent.prepare,audio_preparation:intent.audio_preparation,reviewed_opus_import:intent.reviewed_opus_import,expected_packages:intent.packages.length,expected_files:intent.packages.reduce((n,p)=>n+p.package.files.length,0)})});
  intent={...intent,id:saved.id};await saveDraft(pendingKey,intent);signal.throwIfAborted();setPending(intent);onQuery({item:saved.id});signal.throwIfAborted();setSession(saved);
  if(saved.state==='draft'){for(let i=saved.package_count;i<intent.packages.length;){let end=i;let count=0;let size=0;while(end<intent.packages.length&&end-i<64){const p=intent.packages[end];const length=JSON.stringify(p).length;if(end>i&&(count+p.package.files.length>10000||size+length>12*1024*1024))break;count+=p.package.files.length;size+=length;end++;}saved=await api<Session>(`/intakes/${saved.id}/members`,{method:'POST',signal,headers:{'Idempotency-Key':intent.key+':'+i},body:JSON.stringify({expected_revision:saved.revision,packages:intent.packages.slice(i,end)})});signal.throwIfAborted();setSession(saved);i=end;}
   saved=await api<Session>(`/intakes/${saved.id}/start`,{method:'POST',signal,headers:{'Idempotency-Key':intent.key+':start'},body:JSON.stringify({expected_revision:saved.revision})});signal.throwIfAborted();setSession(saved);
  }
  await saveDraft(pendingKey,undefined);await saveDraft(draftKey,undefined);signal.throwIfAborted();setPending(undefined);setPackages([]);setPlans([]);
  const selectedFiles=files.length?files:originalFiles.get(userId+':'+saved.id)||[];if(intent.packages.some(entry=>entry.package.source.kind==='browser')&&selectedFiles.length){originalFiles.set(userId+':'+saved.id,selectedFiles);void transferBatch(userId,saved.id,selectedFiles);}setTick(v=>v+1);onChanged();
 });}
 const choices=collections.filter(c=>c.content_permissions.can_edit);
 return <section aria-label="Imports workspace">{error&&<p className="notice error" role="alert">{error}</p>}{itemId?<><button onClick={()=>onQuery({item:''})}>Back to imports</button>{session?.id===itemId&&<><h2>Import · {session.package_count} of {session.expected_packages} packages saved</h2><p>{presentationLabel(session.state)} · {session.preserved} originals saved · {session.admitted} accepted · {session.needs_attention} need attention</p>{session.state==='draft'&&<><p>This draft is saved. {session.package_count<session.expected_packages?'The import list is still being saved. Resume saving before starting.':'Review its package pages before starting.'}</p>{pending?.id===session.id&&<button disabled={busy} onClick={()=>void start()}>Resume saving and start import</button>}<button disabled={busy||!session.package_count||session.package_count!==session.expected_packages||session.file_count!==session.expected_files} onClick={()=>void mutate(async()=>{setSession(await api<Session>(`/intakes/${session.id}/start`,{method:'POST',body:JSON.stringify({expected_revision:session.revision})}));setTick(v=>v+1);})}>Start saved import</button></>}{!['completed','cancelled'].includes(session.state)&&<button disabled={busy} onClick={()=>void mutate(async()=>{setSession(await api<Session>(`/intakes/${session.id}/cancel`,{method:'POST'}));transfers.get(userId+':'+session.id)?.controller.abort();})}>Cancel remaining work</button>}{admissionJob?.state==='failed'&&<p role="alert" className="notice error">Could not start some imports: {admissionJob.error}. Accepted packages and their originals remain available. {admissionJob.retryable&&<button disabled={busy} onClick={()=>void mutate(async()=>{await api(`/jobs/${admissionJob.id}/retry`,{method:'POST'});setTick(v=>v+1);})}>Retry remaining imports</button>}</p>}{transfer?.progress&&<UploadMeter value={transfer.progress}/>}{members.some(m=>m.plan.source.kind==='browser')&&<><p className="notice">Keep this window open until uploading finishes. Processing continues afterward.</p>{transfer?.error&&<p role="alert" className="notice error">{transfer.error}</p>}<label>Reselect original folder to resume the complete batch<input type="file" multiple {...{webkitdirectory:''}} onChange={e=>{const f=Array.from(e.target.files||[]);originalFiles.set(userId+':'+session.id,f);void transferBatch(userId,session.id,f);setTick(v=>v+1);}}/></label><label>Reselect package files to resume<input type="file" multiple onChange={e=>{const f=Array.from(e.target.files||[]);originalFiles.set(userId+':'+session.id,f);void transferBatch(userId,session.id,f);setTick(v=>v+1);}}/></label><button disabled={transfer?.running||!originalFiles.has(userId+':'+session.id)} onClick={()=>{void transferBatch(userId,session.id,originalFiles.get(userId+':'+session.id)!);setTick(v=>v+1);}}>Resume selected files</button></>}
 {detail&&<div className="modal-backdrop"><section className="modal curation-modal import-review" role="dialog" aria-modal="true" aria-labelledby="import-review-heading" aria-busy={busy} ref={reviewDialog.ref}><button className="close" aria-label="Close import review" onClick={reviewDialog.dismiss}>×</button><h2 id="import-review-heading">Import review</h2><GuidedIngest key={detail} initialImportId={detail} historyOnly collectionId={session.collection_id} busy={busy} setBusy={setBusy} onPublished={()=>onChanged()} onError={fail}/></section></div>}<ImportedMembers key={session.id} id={session.id} members={members} cursor={cursor} loading={loadingMembers} busy={busy}
  onMore={()=>{setError('');setLoadingMembers(true);setPageCount(loadedPages+1);setTick(value=>value+1);}}
  onChanged={onChanged} onSessionExpired={onSessionExpired}>{m=><><div className="row-text"><strong>{m.plan.default_title}</strong><small>{m.plan.files.length} files</small>{m.import?<IngestStatus summary={m.import}/>:<small>Waiting to start</small>}{m.error&&<><p role="alert">{m.error}</p><button disabled={busy} onClick={()=>void mutate(async()=>{setSession(await api<Session>(`/intakes/${session.id}/members/${m.position}/destination`,{method:'POST',body:JSON.stringify({expected_revision:session.revision,destination:{destination:'separate'}})}));setTick(v=>v+1);})}>Save as a separate entry</button></>}</div>{m.import_id&&<button aria-expanded={detail===m.import_id} onClick={()=>setDetail(m.import_id!)}>Review import</button>}{m.import?.target_item_id&&<button onClick={()=>onMetadata(m.import!.target_item_id!,session.collection_id)}>Review metadata</button>}</>}</ImportedMembers></>}</>:<><h2>Choose source</h2>{pending&&<p className="notice">A reviewed manifest is waiting to finish saving. <button disabled={sourceBusy} onClick={()=>void start()}>Resume reviewed import</button></p>}{!collection?<label>Collection<select value="" onChange={e=>onQuery({collection:e.target.value})}><option value="">Choose a collection</option>{choices.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</select></label>:<><label>Source<select value={source} disabled={reviewLocked} onChange={e=>setSource(e.target.value as typeof source)}><option value="browser">Browser folder / files</option><option value="inbox">Server inbox</option></select></label>{source==='browser'?<>{needsReviewedFiles&&<label>{reviewedFolder?'Reselect folder for reviewed import':'Reselect files for reviewed import'}<input type="file" multiple {...(reviewedFolder?{webkitdirectory:''}:{})} disabled={sourceBusy} onChange={event=>reselectReviewedFiles(Array.from(event.target.files||[]))}/></label>}{musicCollection&&<label>Folder layout<select aria-label="Folder layout" value={folderLayout} disabled={reviewLocked} onChange={event=>{const layout=event.target.value as FolderLayout;if(files[0]?.webkitRelativePath)void chooseFiles(files,true,layout);else setFolderLayout(layout);}}><option value="auto">Best guess</option><option value="items">Separate top-level albums / discs</option><option value="album">One album in the selected folder</option></select></label>}<label>Choose folders<input type="file" multiple {...{webkitdirectory:''}} disabled={reviewLocked} onChange={e=>void chooseFiles(Array.from(e.target.files||[]),true)}/></label><label>Choose one package's files<input type="file" multiple disabled={reviewLocked} onChange={e=>void chooseFiles(Array.from(e.target.files||[]),false)}/></label></>:<><label>Inbox<select value={label} disabled={sourceBusy} onChange={e=>setLabel(e.target.value)}>{inboxes.map(i=><option key={i.label}>{i.label}</option>)}</select></label><InboxPackageChooser collectionId={collection} label={label} multiple selected={selectedInbox} onSelect={setSelectedInbox} disabled={sourceBusy}/><button disabled={reviewLocked||!selectedInbox.length} onClick={()=>void previewInbox()}>Review packages</button></>}
 {!!packages.length&&<><h2>Review packages and groups</h2><p>{packages.length} packages · {packages.reduce((n,p)=>n+p.package.files.length,0)} files · {bytes(packages.reduce((n,p)=>n+p.package.files.reduce((v,f)=>v+f.bytes,0),0))} originals</p>{musicCollection&&<ul><li>Use the same album group label for discs of one album. The title stays automatic unless you enter a manual title.</li><li>Use matching physical-copy labels for one purchased copy, and different labels for separate copies. Copy labels are not disc numbers.</li><li>Keep alternate formats together. Album playback prefers FLAC, then WAV, then MP3.</li><li>Review files and their order before starting.</li></ul>}<DestinationChooser collectionId={collection} selected={destination} onSelect={setDestination} disabled={busy}/><div className="striped-imports">{packages.slice(reviewPage*32,reviewPage*32+32).map((p,index)=>{const pos=reviewPage*32+index;const plan=plans[pos];const group=p.destination.destination==='album_group'?p.destination:null;return <div className="care-row intake-review-row" key={`${reviewVersion}:${pos}`}><div className="row-text"><strong>{plan?.default_title||p.package.source.package_name}</strong><small>{p.package.files.length} files · {plan?.analysis_available?'analysis available':'save originals only'}{musicCollection&&<> · {plan?.preparation_available?'FLAC available':plan?.preparation_reason}</>}</small>{plan?.music_layout?.needs_attention&&!p.package.album_tracks&&<p className="notice">Needs attention · Review album playback files</p>}{plan?.blockers.map(b=><p key={b.message}>{b.message}</p>)}{plan?.music_layout?<ImportFileTree files={p.package.files} layout={plan.music_layout} tracks={p.package.album_tracks??plan.music_layout.tracks} reviewed={p.package.album_tracks!=null} disabled={reviewLocked}
 onTracks={album_tracks=>setPackages(current=>current.map((entry,i)=>i===pos?{...entry,package:{...entry.package,album_tracks}}:entry))}
 onMove={fileIndex=>setPackages(current=>current.map((entry,i)=>{if(i!==pos||!fileIndex)return entry;const files=[...entry.package.files];[files[fileIndex-1],files[fileIndex]]=[files[fileIndex],files[fileIndex-1]];return {...entry,package:{...entry.package,files}};}))}
 onVisibility={(fileIndex,privateFile)=>setPackages(current=>current.map((entry,i)=>i===pos?{...entry,package:{...entry.package,files:entry.package.files.map((file,j)=>j===fileIndex?{...file,visibility:privateFile?'curators':'shared'}:file)}}:entry))}/>:<details><summary>Files and analysis</summary>{!!plan?.anchors.length&&<label>Analysis anchor<select aria-label={`Analysis anchor for ${p.package.source.package_name}`} disabled={reviewLocked} value={p.package.anchor_file_index??''} onChange={e=>void chooseAnchor(pos,e.target.value===''?null:Number(e.target.value))}><option value="">Choose an analysis anchor</option>{plan.anchors.map(a=><option key={a.file_index} value={a.file_index}>{a.label} · {p.package.files[a.file_index]?.name}</option>)}</select></label>}{p.package.files.map((f,fileIndex)=><label key={f.name}><input type="checkbox" disabled={busy||!!pending} checked={f.visibility==='curators'} onChange={e=>setPackages(v=>v.map((entry,i)=>i===pos?{...entry,package:{...entry.package,files:entry.package.files.map((file,j)=>j===fileIndex?{...file,visibility:e.target.checked?'curators':'shared'}:file)}}:entry))}/> Curators only: {f.name} · {bytes(f.bytes)}</label>)}{p.package.files.length>1&&p.package.anchor_file_index==null&&p.package.files.some(f=>/\.(flac|mp3|wav)$/i.test(f.name))&&<div aria-label="Review file order">{p.package.files.map((file,j)=><div key={file.name}><span>{j+1}. {file.name}</span><button disabled={busy||!!pending||j===0} aria-label={`Move ${file.name} earlier`} onClick={()=>setPackages(entries=>entries.map((entry,i)=>{if(i!==pos)return entry;const files=[...entry.package.files];[files[j-1],files[j]]=[files[j],files[j-1]];return {...entry,package:{...entry.package,files}};}))}>Move earlier</button></div>)}</div>}</details>}{musicCollection&&p.package.files.some(f=>/\.(flac|mp3|wav)$/i.test(f.name))&&p.package.anchor_file_index==null&&!p.package.files.some(f=>/\.(cue|toc)$/i.test(f.name))&&<PurchaseFields value={p.package.purchase} disabled={busy||!!pending} onChange={purchase=>setPackages(v=>v.map((entry,i)=>i===pos?{...entry,package:{...entry.package,purchase}}:entry))}/>}</div>{musicCollection&&<label>Album group<input disabled={reviewLocked} value={group?.key||''} onChange={e=>{const key=e.target.value;setPackages(v=>v.map((a,i)=>{if(i!==pos)return a;const matching=v.find(other=>other.destination.destination==='album_group'&&other.destination.key===key)?.destination;return {...a,destination:key?{destination:'album_group',key,title:matching?.destination==='album_group'?matching.title:null,copy_key:group?.copy_key??(digitalPackage(p)?'Digital purchase':'')}:{destination:'separate'}};}));}}/></label>}{group&&<label>Manual album title (optional)<input disabled={reviewLocked} value={group.title||''} placeholder="Use automatic metadata" onChange={e=>groupTitle(group.key,e.target.value)}/></label>}{p.destination.destination!=='separate'&&!digitalPackage(p)&&<label>Physical copy label<input disabled={reviewLocked} placeholder="For example: purchased copy A" value={p.destination.copy_key} onChange={e=>{const copy_key=e.target.value;setPackages(v=>v.map((a,i)=>i===pos&&a.destination.destination!=='separate'?{...a,destination:{...a.destination,copy_key}}:a));}}/></label>}<button disabled={reviewLocked||!destination} onClick={()=>{if(destination)setPackages(v=>v.map((a,i)=>i===pos?{...a,destination:{destination:'existing',item_id:destination.id,expected_revision:destination.revision,copy_key:digitalPackage(p)?'Digital purchase':''}}:a));}}>Attach to selected {musicCollection?'album':'entry'}</button><small>Destination: {p.destination.destination==='existing'?destination?.title||p.destination.item_id:group?`Album group ${group.key} · ${group.title||'automatic title'}`:'Separate entry'}</small></div>;})}</div><div><button disabled={!reviewPage} onClick={()=>setReviewPage(v=>v-1)}>Previous packages</button> Page {reviewPage+1} of {Math.ceil(packages.length/32)} <button disabled={(reviewPage+1)*32>=packages.length} onClick={()=>setReviewPage(v=>v+1)}>Next packages</button></div>{!!unassigned.length&&<details><summary>{unassigned.length} unsupported or unassigned files will remain outside this import</summary>{unassigned.slice(unassignedPage*100,unassignedPage*100+100).map(n=><p key={n}>{n}</p>)}<button disabled={!unassignedPage} onClick={()=>setUnassignedPage(v=>v-1)}>Previous files</button> Page {unassignedPage+1} of {Math.ceil(unassigned.length/100)} <button disabled={(unassignedPage+1)*100>=unassigned.length} onClick={()=>setUnassignedPage(v=>v+1)}>Next files</button></details>}<div className="care-actions intake-actions"><span role="status">{savedDraft===draft?'Import review saved':'Saving import review…'}</span><label><input type="checkbox" checked={automatic} onChange={e=>setAutomatic(e.target.checked)}/> Automatic metadata and artwork</label>{musicCollection&&<ImportPreparation value={audioChoice} onChange={setAudioChoice} settings={opusDefault} packages={plans} canFlac={plans.some(plan=>plan.preparation_available)} disabled={reviewLocked}/>}{missingAlbumSelection&&<p className="notice">Expand the rows marked Needs attention and choose their album playback files.</p>}{missingCopy&&<p className="notice">Choose a physical-copy label for each grouped disc before starting.</p>}<button className="primary" disabled={busy||missingCopy||missingAlbumSelection||hydrated!==draftKey||needsReviewedFiles||!pending&&musicCollection&&audioChoice==='default'&&opusDefault===undefined} onClick={()=>void start()}>Start import</button></div></>}</>}
 <SavedImports key={`${userId}:${collection}`} collection={collection} onOpen={s=>onQuery({item:s.id,collection:s.collection_id})} onSessionExpired={onSessionExpired}/></>}{busy&&<p role="status">Saving import…</p>}</section>;
}
