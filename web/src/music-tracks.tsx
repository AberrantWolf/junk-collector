import {PreparationFormat,conversionProfile,preparationQuery,profileLabel,type PreparationPreset} from './audio-profiles';
import AccurateRipCheck from './accuraterip-check';
import PurchaseDetails from './purchase-details';
import ActionMenu from './action-menu';
import InfoHelp from './info-help';
import {presentationLabel} from './presentation-labels';
import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage,addMusicTracks,startMusicTracks,retainUnchanged} from '@collector/runtime';
import type {components} from './generated/api';

type Availability=components['schemas']['MusicTrackAvailability'];
type AlbumPlan=components['schemas']['MusicAlbumPreparationPlan'];
type Batch=components['schemas']['MusicPreparationBatchView'];
type Coverage=components['schemas']['MusicAlbumCoverage'];
type DiscSource=components['schemas']['MusicDiscSourceCoverage'];
type Review=components['schemas']['MetadataReviewView'];
type Item=components['schemas']['PresentedItem'];
type Props={preparationPreset?:PreparationPreset;onPreparationPreset?:(preset:PreparationPreset)=>void;preparationSources?:string[];onPreparationSources?:(sources:string[])=>void;item:Item;canPrepare:boolean;onSessionExpired:()=>void;onPrepared?:(item:Item)=>void;onOpenHistory?:()=>void;onOpenFiles?:()=>void;onOpenMetadata?:()=>void;onAddToList?:(source?:string,track?:number)=>void;onOpenAnalysis?:(sourceId:string,operation:'structure'|'accuraterip'|'files')=>void};


function stateLabel(row:Availability):string{
 if(row.state.status==='available')return `${(row.state.length/1048576).toFixed(1)} MiB`;
 return row.state.status==='preparing'?'Preparing…':row.state.reason;
}
function mediaUrl(itemId:string,row:Availability):string|null{
 return row.state.status==='available'?`/api/v1/items/${itemId}/representations/${row.state.representation_id}/${row.state.file_index}`:null;
}
function canDecode(row:Availability):boolean{
 return row.state.status==='available'&&document.createElement('audio').canPlayType(row.state.media_type)!=='';
}
function preferredAvailable(group:Availability[]):Availability|undefined{
 const available=group.filter(row=>row.state.status==='available'&&canDecode(row));
 return available.find(row=>!row.profile)||available.find(row=>row.profile?.codec==='flac')||available[0];
}
function discKey(row:Availability):string{return `${row.capture_id}/${row.source_representation_id}`;}
function trackKey(row:Availability):string{return `${discKey(row)}/${row.track_position}`;}
function discPosition(coverage:Coverage|undefined,key:string):number{
 return coverage?.sources.find(source=>`${source.capture_id}/${source.source_representation_id}`===key)?.medium_position??Number.MAX_SAFE_INTEGER;
}
function object(value:unknown):Record<string,unknown>{return value&&typeof value==='object'&&!Array.isArray(value)?value as Record<string,unknown>:{};}
function discStatus(source:DiscSource):string{
 switch(source.match_state){
  case 'disc_id_matched':return `Disc ID matches album disc ${source.medium_position}.`;
  case 'curator_assigned':return `Assigned to album disc ${source.medium_position}.`;
  case 'single_medium':return 'Matches the only album disc.';
  case 'no_release':return 'Choose an album release first.';
  case 'no_layout':return 'CD track layout not recorded.';
  case 'needs_assignment':return 'Choose which album disc this is.';
  case 'disc_id_not_listed':return 'This disc ID isn’t listed for the chosen release.';
  case 'ambiguous_disc_id':return 'This disc ID matches more than one album disc.';
  case 'assignment_conflicts_with_disc_id':return 'The chosen disc doesn’t match the disc ID, so track names aren’t applied.';
  case 'track_layout_mismatch':return 'The tracks don’t match that album disc, so track names aren’t applied.';
 }
}
function verificationStatus(source:DiscSource):string{return source.audio_verification?`AccurateRip: ${presentationLabel(source.audio_verification)}`:'AccurateRip: not checked';}
function DiscCoverageSource({source,coverage,canAssign,busy,onAssign,onOpenMetadata,onOpenAnalysis,itemId,onChecked}:{itemId:string;onChecked:()=>void;source:DiscSource;coverage:Coverage;canAssign:boolean;busy:boolean;onAssign:(position:number)=>void;onOpenMetadata?:()=>void;onOpenAnalysis?:(sourceId:string,operation:'structure'|'accuraterip'|'files')=>void}){
 const issue=!['disc_id_matched','curator_assigned','single_medium'].includes(source.match_state);
 return <div className="disc-source"><div className="disc-source-heading"><strong>Original import · {source.file_count} {source.file_count===1?'file':'files'}</strong><small title={source.file_label}>{source.file_label} · Added {new Date(source.captured_at).toLocaleDateString()}</small></div><p className={issue?'disc-status issue':'disc-status'}>{discStatus(source)}</p><dl><dt>Disc details</dt><dd><details><summary>Technical details</summary>CD disc ID: {source.recorded_disc_id||'Not available'}</details></dd><dt>CD structure</dt><dd>{source.track_count==null?'Not recorded':`${source.track_count} tracks recorded`}</dd><dt>Audio check</dt><dd>{verificationStatus(source)}</dd></dl>
  {canAssign&&coverage.release_id&&(coverage.media.length>1||source.assigned_medium_position!=null)&&<label className="disc-assignment">Album disc<select aria-label={`Album disc for ${source.file_label}`} value={source.assigned_medium_position||''} disabled={busy} onChange={event=>onAssign(Number(event.target.value))}><option value="">Automatic / not assigned</option>{[...coverage.media].sort((a,b)=>a.position-b.position).map(medium=><option key={medium.position} value={medium.position}>Disc {medium.position} · {medium.track_count} tracks</option>)}</select></label>}
  <div className="disc-next-action">{source.track_count==null&&onOpenAnalysis&&<button onClick={()=>onOpenAnalysis(source.source_representation_id,'structure')}>Inspect CD structure</button>}{['no_release','disc_id_not_listed','track_layout_mismatch'].includes(source.match_state)&&onOpenMetadata&&<button onClick={onOpenMetadata}>Review album release</button>}{canAssign&&<AccurateRipCheck itemId={itemId} sourceId={source.source_representation_id} onComplete={onChecked}/>}</div>
 </div>;
}

export default function MusicTracks({preparationPreset,onPreparationPreset,preparationSources,onPreparationSources,item,canPrepare:canPrepareRequested,onSessionExpired,onPrepared,onOpenHistory,onOpenFiles,onOpenMetadata,onOpenAnalysis,onAddToList}:Props){
 const [localPreset,setLocalPreset]=useState<PreparationPreset>('flac');
 const preset=preparationPreset??localPreset;
 const profile=conversionProfile(preset);
 const formatLabel=profileLabel(profile);
 const [rows,setRows]=useState<Availability[]>([]);
 const trackCache=useRef<{revision:string;rows:Availability[]}|undefined>(undefined);
 const canPrepare=canPrepareRequested&&(!item.digital_audio||rows.some(row=>!!row.profile));
 const [coverage,setCoverage]=useState<Coverage>();
 const [assignmentBusy,setAssignmentBusy]=useState<string>();
 const [editingTrack,setEditingTrack]=useState<{source:string;position:number;title:string;disc:string;number:string;albumSelected:boolean}>();
 // Captures are independent records; publication need not revise the Item.
 const captureRevisions=item.captures.map(capture=>`${capture.id}:${capture.revision}`).join(',');
 const [loading,setLoading]=useState(true);
 const [error,setError]=useState('');
 const [queueNotice,setQueueNotice]=useState('');
 const [playBusy,setPlayBusy]=useState(false);
 const playback=useRef<AbortController|null>(null);
 const [busy,setBusy]=useState(false);
 const [pendingTracks,setPendingTracks]=useState<Set<string>>(new Set());
 const [loadedPlan,setPlan]=useState<AlbumPlan>();
 const plan=profileLabel(loadedPlan?.profile)===formatLabel?loadedPlan:undefined;
 const [batches,setBatches]=useState<Batch[]>([]);
 const [discSources,setDiscSources]=useState<Record<string,string>>({});
 const [includedDiscs,setIncludedDiscs]=useState<Set<string>>(new Set());
 const [refresh,setRefresh]=useState(0);
 const [availabilityRefresh,setAvailabilityRefresh]=useState(0);
 const callbacks=useRef({onSessionExpired,onPrepared});callbacks.current={onSessionExpired,onPrepared};
 const requests=useRef(new AbortController());
 const commandKeys=useRef(new Map<string,string>());
 const initializedDiscs=useRef(new Set<string>());
 const notifiedOutputs=useRef<string|undefined>(undefined);
 function commandKey(scope:string){const keys=commandKeys.current;const key=keys.get(scope)||crypto.randomUUID();keys.set(scope,key);return key;}
 function report(value:unknown){
  if(isApiRequestError(value)&&value.code==='unauthenticated')callbacks.current.onSessionExpired();
  else setError(requestErrorMessage(value,'You no longer have access to this music.'));
 }
 useEffect(()=>{
  requests.current=new AbortController();commandKeys.current.clear();initializedDiscs.current.clear();notifiedOutputs.current=undefined;
  trackCache.current=undefined;setRows([]);setCoverage(undefined);setError('');setQueueNotice('');setPlayBusy(false);setPendingTracks(new Set());setPlan(undefined);setBatches([]);setDiscSources({});setIncludedDiscs(new Set());setBusy(false);
  return()=>{requests.current.abort();playback.current?.abort();};
 },[item.id]);

 // Availability is viewer-safe and resumes even when another user owns the Job.
 useEffect(()=>{
  const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;setLoading(true);
  async function load(){
   try{
    const summary=await api<components['schemas']['TrackSummary']>(`/items/${item.id}/client-tracks/summary`,{signal:controller.signal});
    if(controller.signal.aborted)return;
    if(summary.pending){timer=setTimeout(load,1500);return;}
    let next=trackCache.current?.revision===summary.tracks_revision?trackCache.current.rows:undefined;
    if(!next){
     next=[];let cursor:number|undefined;
     do {
      const query=new URLSearchParams({revision:summary.tracks_revision});if(cursor!==undefined)query.set('cursor',String(cursor));
      const page=await api<components['schemas']['TrackPage']>(`/items/${item.id}/client-tracks?${query}`,{signal:controller.signal});
      if(controller.signal.aborted)return;if(page.summary.pending||page.summary.tracks_revision!==summary.tracks_revision)throw new Error('Album tracks changed; retrying');
      next.push(...page.items);cursor=page.next_cursor??undefined;
     }while(cursor!==undefined);
     trackCache.current={revision:summary.tracks_revision,rows:next};
    }
    const nextCoverage=item.supports_listening?await api<Coverage>(`/items/${item.id}/music-disc-coverage`,{signal:controller.signal}):undefined;
    if(controller.signal.aborted)return;setRows(next);setCoverage(current=>retainUnchanged(current,nextCoverage));setLoading(false);
    const outputs=JSON.stringify(next.flatMap(row=>row.state.status==='available'?[row.state.representation_id]:[]).sort());
    if(callbacks.current.onPrepared&&outputs!==notifiedOutputs.current&&outputs!=='[]'){
     const saved=await api<Item>(`/items/${item.id}`,{signal:controller.signal});
     if(controller.signal.aborted)return;
     notifiedOutputs.current=outputs;callbacks.current.onPrepared(saved);
    }
    timer=setTimeout(load,next.some(row=>row.state.status==='preparing')?1500:5000);
   }catch(value){if(!controller.signal.aborted){setLoading(false);if(isApiRequestError(value)&&(value.code==='forbidden'||value.code==='unauthenticated'||value.status===404)){setRows([]);}report(value);if(!isApiRequestError(value)||!['forbidden','unauthenticated'].includes(value.code)&&value.status!==404)timer=setTimeout(load,5000);}}
  }
  void load();return()=>{controller.abort();clearTimeout(timer);};
 },[item.id,item.revision,captureRevisions,availabilityRefresh]);

 useEffect(()=>{
  if(!canPrepare)return;const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;
  async function load(){
   try{
    const [nextPlan,nextBatches]=await Promise.all([
     api<AlbumPlan>(`/items/${item.id}/music-preparation-plan${preset==='flac'?'':'?'+preparationQuery(preset)}`,{signal:controller.signal,operation:'Load track preparation choices'}),
     api<Batch[]>(`/items/${item.id}/music-preparation-batches`,{signal:controller.signal,operation:'Load track preparation results'}),
    ]);if(controller.signal.aborted)return;
    setPlan(current=>retainUnchanged(current,nextPlan));setBatches(current=>retainUnchanged(current,nextBatches));
    setDiscSources(current=>{const next={...current};for(const disc of nextPlan.discs){const selected=disc.sources.find(source=>preparationSources?.includes(source.source_representation_id))??disc.sources.find(source=>source.default_selected);if(!disc.sources.some(source=>source.source_representation_id===next[disc.capture_id]))next[disc.capture_id]=selected?.source_representation_id||'';}return retainUnchanged(current,next);});
    const added=nextPlan.discs.filter(disc=>!initializedDiscs.current.has(disc.capture_id)).map(disc=>disc.capture_id);
    for(const id of added)initializedDiscs.current.add(id);
    if(added.length)setIncludedDiscs(current=>new Set([...current,...added]));
    timer=setTimeout(load,nextPlan.discs.some(disc=>disc.sources.some(source=>source.preparing>0))||nextBatches.some(batch=>batch.queued>0||batch.running>0)?1200:5000);
   }catch(value){if(!controller.signal.aborted){report(value);if(!isApiRequestError(value)||(!['forbidden','unauthenticated'].includes(value.code)&&value.status!==404))timer=setTimeout(load,5000);}}
  }
  void load();return()=>{controller.abort();clearTimeout(timer);};
 },[canPrepare,item.id,item.revision,captureRevisions,refresh,preset]);

 async function prepare(row:Availability){
  const controller=requests.current;const key=`${trackKey(row)}/${profileLabel(row.profile)}`;setPendingTracks(current=>new Set(current).add(key));setError('');
  try{
   await api<{job_id:string}>(`/items/${item.id}/captures/${row.capture_id}/music-tracks/${row.track_position}/prepare`,{method:'POST',signal:controller.signal,headers:{'Idempotency-Key':commandKey(key)},body:JSON.stringify({source_representation_id:row.source_representation_id,profile:row.profile})});
   if(controller.signal.aborted)return;commandKeys.current.delete(key);
   setRows(current=>current.map(value=>trackKey(value)===trackKey(row)&&profileLabel(value.profile)===profileLabel(row.profile)?{...value,can_prepare:false,state:{status:'preparing'}}:value));
   setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);
  }catch(value){if(!controller.signal.aborted)report(value);}finally{if(!controller.signal.aborted)setPendingTracks(current=>{const next=new Set(current);next.delete(key);return next;});}
 }

 async function prepareAlbum(){
  if(!plan||!selectedMissing||selectedActive)return;const sources=plan.discs.filter(disc=>includedDiscs.has(disc.capture_id)).map(disc=>discSources[disc.capture_id]).filter(Boolean);
  const controller=requests.current;const key=`album:${preset}:${JSON.stringify([...sources].sort())}`;
  setBusy(true);setError('');try{
   const batch=await api<Batch>(`/items/${item.id}/music-preparation-batches`,{method:'POST',signal:controller.signal,headers:{'Idempotency-Key':commandKey(key)},body:JSON.stringify({profile,source_representation_ids:sources})});
   if(controller.signal.aborted)return;commandKeys.current.delete(key);
   setRows(current=>current.map(row=>profileLabel(row.profile)===formatLabel&&row.state.status!=='available'&&batch.members.some(member=>member.source_representation_id===row.source_representation_id&&member.track_position===row.track_position&&['queued','running'].includes(member.state))?{...row,can_prepare:false,state:{status:'preparing'}}:row));
   setBatches(current=>[batch,...current.filter(value=>value.batch_id!==batch.batch_id)]);setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);
  }catch(value){if(!controller.signal.aborted)report(value);}finally{if(!controller.signal.aborted)setBusy(false);}
 }
 async function batchAction(batch:Batch,action:'cancel'|'retry'){
  const controller=requests.current;const key=`${batch.batch_id}:${action}`;
  setBusy(true);setError('');try{
   const next=await api<Batch>(`/music-preparation-batches/${batch.batch_id}/${action}`,{method:'POST',signal:controller.signal,headers:{'Idempotency-Key':commandKey(key)}});
   if(controller.signal.aborted)return;commandKeys.current.delete(key);
   setBatches(current=>current.map(value=>value.batch_id===next.batch_id?next:value));setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);
  }catch(value){if(!controller.signal.aborted)report(value);}finally{if(!controller.signal.aborted)setBusy(false);}
 }

 async function assignDisc(source:DiscSource,position:number){
  if(!coverage?.release_id)return;
  setAssignmentBusy(source.source_representation_id);setError('');
  try{
   const review=await api<Review>(`/items/${item.id}/metadata`,{signal:requests.current.signal});
   const previous=review.authority.domain_overrides;
   const payload=object(previous?.payload);
   const assignments=Array.isArray(payload.disc_assignments)?payload.disc_assignments.filter(value=>object(value).release_id===coverage.release_id&&object(value).source_representation_id!==source.source_representation_id):[];
   if(position)assignments.push({source_representation_id:source.source_representation_id,release_id:coverage.release_id,medium_position:position});
   await api<Review>(`/items/${item.id}/metadata`,{method:'PUT',signal:requests.current.signal,body:JSON.stringify({expected_revision:review.revision,change:{change:'set_domain_overrides',overrides:{module:'org.junk.music',schema_version:previous?.schema_version??item.domain.schema_version,payload:{...payload,disc_assignments:assignments}}}})});
   setAvailabilityRefresh(value=>value+1);
   if(callbacks.current.onPrepared)callbacks.current.onPrepared(await api<Item>(`/items/${item.id}`,{signal:requests.current.signal}));
  }catch(value){if(!requests.current.signal.aborted)report(value);}finally{if(!requests.current.signal.aborted)setAssignmentBusy(undefined);}
 }

 async function saveTrack(){
  if(!editingTrack)return;const edit=editingTrack;const controller=requests.current;setBusy(true);setError('');
  try{const review=await api<Review>(`/items/${item.id}/metadata`,{signal:controller.signal});
   const previous=review.authority.domain_overrides;const payload=object(previous?.payload);
   const track_overrides=Array.isArray(payload.track_overrides)?payload.track_overrides.filter(value=>object(value).source_representation_id!==edit.source||object(value).track_position!==edit.position):[];
   track_overrides.push({source_representation_id:edit.source,track_position:edit.position,title:edit.title.trim()||null,disc_position:edit.disc?Number(edit.disc):null,display_position:edit.number?Number(edit.number):null,album_selected:edit.albumSelected});
   await api<Review>(`/items/${item.id}/metadata`,{method:'PUT',signal:controller.signal,body:JSON.stringify({expected_revision:review.revision,change:{change:'set_domain_overrides',overrides:{module:item.domain.module,schema_version:previous?.schema_version??item.domain.schema_version,payload:{...payload,track_overrides}}}})});
   if(controller.signal.aborted)return;setEditingTrack(undefined);setAvailabilityRefresh(v=>v+1);if(callbacks.current.onPrepared)callbacks.current.onPrepared(await api<Item>(`/items/${item.id}`,{signal:controller.signal}));
  }catch(value){if(!controller.signal.aborted)report(value);}finally{if(!controller.signal.aborted)setBusy(false);}
 }
 const discs=React.useMemo(()=>{
  const grouped=new Map<string,Map<number,Availability[]>>();
  for(const row of rows){const key=discKey(row);const tracks=grouped.get(key)||new Map<number,Availability[]>();const variants=tracks.get(row.track_position)||[];variants.push(row);tracks.set(row.track_position,variants);grouped.set(key,tracks);}
  return [...grouped.entries()].map(([key,tracks])=>[key,[...tracks.entries()].sort(([,a],[,b])=>(a[0].disc_position??1)-(b[0].disc_position??1)||(a[0].display_position??a[0].track_position)-(b[0].display_position??b[0].track_position)).map(([,variants])=>variants)] as const).sort(([a],[b])=>discPosition(coverage,a)-discPosition(coverage,b)||a.localeCompare(b));
 },[rows,coverage]);
 const orderedMedia=[...(coverage?.media||[])].sort((a,b)=>a.position-b.position);
 const orderedPlanDiscs=[...(plan?.discs||[])].sort((a,b)=>{
  const selected=(disc:AlbumPlan['discs'][number])=>disc.sources.find(source=>source.source_representation_id===discSources[disc.capture_id])||disc.sources.find(source=>source.default_selected);
  const position=(disc:AlbumPlan['discs'][number])=>{
   const source=selected(disc);
   return source?discPosition(coverage,`${source.capture_id}/${source.source_representation_id}`):Number.MAX_SAFE_INTEGER;
  };
  return position(a)-position(b)||a.capture_id.localeCompare(b.capture_id);
 });
 const selectedSources=(plan?.discs||[]).filter(disc=>includedDiscs.has(disc.capture_id)).flatMap(disc=>disc.sources.filter(source=>source.source_representation_id===discSources[disc.capture_id]));
 const selectedMissing=selectedSources.reduce((count,source)=>count+source.eligible,0);
 const selectedActive=selectedSources.some(source=>source.preparing>0&&source.already_available+source.preparing===source.track_count||batches.some(batch=>batch.members.some(member=>member.source_representation_id===source.source_representation_id&&['queued','running'].includes(member.state))));
 const allReady=!!plan?.discs.length&&plan.discs.every(disc=>{const source=disc.sources.find(source=>source.source_representation_id===discSources[disc.capture_id]);return source&&source.track_count>0&&source.already_available===source.track_count;});
 function preparationSourceLabel(id:string){
  const capture=item.captures.find(c=>c.representations.some(r=>r.id===id));
  const source=capture?.representations.find(r=>r.id===id);
  const files=source?.files||[];
  return files.length?`${files[0].name}${files.length>1?` and ${files.length-1} more`:''} · ${new Date(capture!.created_at).toLocaleString()}`:'Saved original';
 }

 function playableRow(group:Availability[]):Availability|undefined{
  return preferredAvailable(group)||group.find(row=>row.state.status==='available')||group.find(row=>row.can_stream_original);
 }
 function queuedTrack(row:Availability){
  const saved=row.state.status==='available'?row.state:undefined;
  return {itemId:item.id,collectionId:item.collection_id,album:item.title,artist:item.subtitle||undefined,title:row.title,position:row.display_position??row.track_position,representationId:saved?.representation_id||row.source_representation_id,fileIndex:saved?.file_index??0,mediaType:saved?.media_type||'',artwork:item.primary_artwork||undefined,original:saved?.representation_id===row.source_representation_id,
   sourceId:row.source_representation_id,sourcePosition:row.track_position,seconds:row.frame_count/(row.audio_format?.sample_rate||44100)||undefined,opusBitrate:row.profile?.codec==='opus'?row.profile.bitrate_kbps:undefined,liveOnly:!saved};
 }
 async function play(source?:string,track?:number){
  playback.current?.abort();const controller=new AbortController();playback.current=controller;
  const signal=AbortSignal.any([controller.signal,requests.current.signal]);setPlayBusy(true);setError('');setQueueNotice('');
  try{
   const freshPromise=api<Availability[]>(`/items/${item.id}/music-tracks`,{signal,operation:'Check playback tracks'});
   const snapshotPromise=track===undefined?api<components['schemas']['AlbumPlaybackSnapshot']>(`/items/${item.id}/client-playback${source?'?source='+encodeURIComponent(source):''}`,{signal,operation:'Load album playback selection'}):Promise.resolve(undefined);
   const [fresh,snapshot]=await Promise.all([freshPromise,snapshotPromise]);signal.throwIfAborted();
   const scope=snapshot?.tracks||fresh.filter(row=>row.source_representation_id===source&&row.track_position===track).slice(0,1);
   if(!scope.length)throw new Error('No playable tracks are ready.');
   const selected=scope.map(target=>{
    const row=playableRow(fresh.filter(row=>row.capture_id===target.capture_id&&row.source_representation_id===target.source_representation_id&&row.track_position===target.track_position));
    if(!row)throw new Error(`Track ${target.track_position} can’t be played in this browser. Your queue is unchanged.`);
    return queuedTrack(row);
   });
   if(!startMusicTracks(selected))throw new Error('The queue holds up to 5,000 tracks. Play a smaller selection.');
   setQueueNotice(`Playing ${selected.length===1?'one track':selected.length+' tracks'}.`);
  }catch(value){if(!signal.aborted)report(value);}finally{controller.abort();if(playback.current===controller){playback.current=null;if(!requests.current.signal.aborted)setPlayBusy(false);}}
 }
 function enqueue(groups:Availability[][],album=false){
  if(album&&groups.some(group=>group.some(row=>row.album_selection==='needs_review'))){setError('Some tracks have more than one version. Choose one with Edit track details first.');return;}
  if(album)groups=groups.filter(group=>group.some(row=>row.album_selection==='included'));
  const tracks=groups.flatMap(group=>{const row=playableRow(group);return row?[queuedTrack(row)]:[];});
  const added=addMusicTracks(tracks);setQueueNotice(added===tracks.length?`Added ${added} ${added===1?'track':'tracks'} to your queue.`:`Added ${added} tracks. The queue is full (5,000 tracks).`);
 }

 if(!discs.length&&!error&&!item.supports_listening)return null;
 return <section className={canPrepare?'music-preparation':'album-tracks'} aria-label={canPrepare?'Prepare audio':'Album tracks'}>
  {canPrepare&&<div className="music-heading">{item.primary_artwork?<img src={`/api/v1/collections/${item.collection_id}/assets/${item.primary_artwork}/thumbnail`} alt={`${item.title} cover`}/>:<div className="music-cover-placeholder" aria-label="No cover artwork">♪</div>}<div><h3>{canPrepare?'Prepare audio':'Album tracks'}</h3><strong>{item.title}</strong>{item.subtitle&&<small>{item.subtitle}</small>}</div></div>}
  {coverage&&item.domain.module==='org.junk.music'&&!item.digital_audio&&!rows.some(row=>!row.profile)&&<details className="album-inspection"><summary>Disc details &amp; checks</summary><section className="album-coverage" aria-label="Album discs"><header><h4>Album discs</h4><span>{coverage.media.filter(medium=>coverage.sources.some(source=>source.medium_position===medium.position)).length} of {coverage.media.length||'?'} linked</span></header>{coverage.release_title?<p className="muted">Selected release: {coverage.release_title}.</p>:<p className="notice">No album release selected. {onOpenMetadata?<button onClick={onOpenMetadata}>Find album and track names</button>:'A curator can choose one.'}</p>}
   {orderedMedia.map(medium=>{const matched=coverage.sources.filter(source=>source.medium_position===medium.position);return <div className="album-medium" key={medium.position}><strong>Disc {medium.position}</strong><span>{medium.track_count} listed tracks · {matched.length?`${matched.length} ${matched.length===1?'original':'originals'} linked`:'No original linked'}</span><details><summary>Technical details</summary><small>Registered disc {medium.registered_disc_ids.length===1?'ID':'IDs'}: {medium.registered_disc_ids.length?medium.registered_disc_ids.join(', '):'None listed for this release'}</small></details>{!medium.track_count&&onOpenMetadata&&<button onClick={onOpenMetadata}>Find a track list</button>}{!matched.length&&onOpenFiles&&<button onClick={onOpenFiles}>Attach this disc</button>}{matched.map(source=><DiscCoverageSource itemId={item.id} onChecked={()=>setRefresh(v=>v+1)} key={source.source_representation_id} source={source} coverage={coverage} canAssign={canPrepare} busy={assignmentBusy===source.source_representation_id} onAssign={position=>void assignDisc(source,position)} onOpenMetadata={onOpenMetadata} onOpenAnalysis={onOpenAnalysis}/>)}</div>;})}
   {coverage.sources.filter(source=>!source.medium_position).map(source=><div className="album-medium unmatched-source" key={source.source_representation_id}><strong>Unmatched original</strong><DiscCoverageSource itemId={item.id} onChecked={()=>setRefresh(v=>v+1)} source={source} coverage={coverage} canAssign={canPrepare} busy={assignmentBusy===source.source_representation_id} onAssign={position=>void assignDisc(source,position)} onOpenMetadata={onOpenMetadata} onOpenAnalysis={onOpenAnalysis}/></div>)}
   {!coverage.sources.length&&<p className="notice">No CD files attached. {onOpenFiles&&<button onClick={onOpenFiles}>Attach original files</button>}</p>}
  </section></details>}
  {loading&&<p role="status">Checking tracks…</p>}
  {!loading&&!discs.length&&!error&&<p className="notice">{item.digital_audio?(item.content_permissions.can_edit?'Purchased files are saved but not ready to play yet. Check Work & history.':'No purchased tracks are ready yet.') : item.captures.length?(item.content_permissions.can_edit?'No playable audio tracks found. Check that the full CD was imported.':'No playable audio tracks found.'):(item.content_permissions.can_edit?'No files attached yet. Use Edit / attach files to add them.':'No files attached yet.')}</p>}
  {item.domain.module==='org.junk.games'&&<>{canPrepareRequested&&onOpenAnalysis&&item.captures.flatMap(c=>c.representations).filter(r=>r.role==='preservation_master').map(r=><button key={r.id} disabled={busy} onClick={()=>onOpenAnalysis(r.id,'structure')}>Inspect game CD audio</button>)}</>}
  {canPrepare&&<InfoHelp label="audio preparation">FLAC is lossless. Opus makes smaller files. Set up encoders in Administration → Audio encoders.</InfoHelp>}
  {error&&<p role="alert" className="error">{error}</p>}
  {canPrepare&&<PreparationFormat value={preset} onChange={onPreparationPreset??setLocalPreset} disabled={busy}/>}
  {canPrepare&&plan&&allReady&&<p className="notice" role="status">All album tracks are ready as {formatLabel}.</p>}
  {canPrepare&&plan&&(!allReady||plan.discs.some(disc=>disc.ambiguous))&&<section className="album-preparation" aria-label="Prepare album"><div className="section-heading"><div><h4>Prepare album as {formatLabel}</h4></div><span>{plan.total_tracks} tracks</span></div>
   {orderedPlanDiscs.map((disc,index)=>{const matched=coverage?.sources.find(source=>source.capture_id===disc.capture_id&&source.source_representation_id===discSources[disc.capture_id]);return <fieldset key={disc.capture_id}><legend>{matched?.medium_position?`Album disc ${matched.medium_position}`:item.digital_audio?`Digital original ${index+1}`:`CD ${index+1} · album disc not assigned`}</legend><label><input type="checkbox" checked={includedDiscs.has(disc.capture_id)} onChange={event=>setIncludedDiscs(current=>{const next=new Set(current);event.target.checked?next.add(disc.capture_id):next.delete(disc.capture_id);return next;})}/> {item.digital_audio?'Include this original':'Include this disc'}</label>{disc.sources.length===1?<p className="muted">{disc.sources[0].track_count} tracks · {disc.sources[0].already_available} ready · {disc.sources[0].preparing} preparing</p>:<>{disc.message&&<p className="notice">{disc.message}</p>}{disc.sources.map((source,sourceIndex)=><label key={source.source_representation_id}><input type="radio" name={`album-source-${disc.capture_id}`} checked={discSources[disc.capture_id]===source.source_representation_id} onChange={()=>{const next={...discSources,[disc.capture_id]:source.source_representation_id};setDiscSources(next);onPreparationSources?.(plan.discs.filter(d=>d.ambiguous).map(d=>next[d.capture_id]).filter(Boolean));}}/> Source {sourceIndex+1} · {preparationSourceLabel(source.source_representation_id)} · {source.track_count} tracks · {source.already_available} ready</label>)}</>}</fieldset>;})}
   {plan.blockers.map(message=><p className="notice" key={message}>{message}</p>)}
   <button className="primary" disabled={busy||selectedActive||!selectedMissing||!plan.can_submit||![...includedDiscs].some(capture=>discSources[capture])||plan.discs.some(disc=>includedDiscs.has(disc.capture_id)&&!discSources[disc.capture_id])} onClick={()=>void prepareAlbum()}>{busy?'Submitting…':selectedActive?'Album preparation in progress…':!selectedMissing?'No missing tracks can be prepared':`Prepare missing ${formatLabel} tracks`}</button>
  </section>}
  {canPrepare&&batches.length>0&&<section className="preparation-batches" aria-label="Album preparation progress"><h4>Preparation history</h4>{batches.length===50&&<p className="muted">Showing the 50 most recent. {onOpenHistory&&<button onClick={onOpenHistory}>Open older preparation work</button>}</p>}{batches.map(batch=><article key={batch.batch_id}><div className="section-heading"><div><strong>Album {profileLabel(batch.profile)} preparation</strong><small>{new Date(batch.created_at).toLocaleString()}</small><details><summary>Technical details</summary>Batch ID: {batch.batch_id}</details></div><span>{batch.succeeded}/{batch.members.length} ready</span></div><p className="muted">{batch.queued} queued · {batch.running} running · {batch.failed} failed · {batch.cancelled} cancelled · {batch.ineligible} cannot prepare{batch.cancelled_by_user?' · batch cancelled':''}</p>{batch.admission_error&&<p className="error" role="alert">Could not start preparation: {batch.admission_error}</p>}<details><summary>Track outcomes</summary>{batch.members.map(member=><p className="batch-member" key={member.member_id}><strong>{member.track_position}. {member.title}</strong><span className={`identity ${member.state}`}>{presentationLabel(member.state)}</span>{member.error&&<small>{member.error}</small>}</p>)}</details><div className="profile-actions">{batch.can_cancel&&<button disabled={busy} onClick={()=>void batchAction(batch,'cancel')}>Cancel outstanding tracks</button>}{batch.can_retry&&<button disabled={busy} onClick={()=>void batchAction(batch,'retry')}>Retry unfinished tracks</button>}</div></article>)}</section>}
  {canPrepareRequested&&item.domain.module==='org.junk.music'&&onOpenAnalysis&&item.captures.flatMap(c=>c.representations).filter(r=>r.role==='preservation_master'&&r.files.some(f=>/\.(wav|flac|mp3)$/i.test(f.name))&&!r.files.some(f=>/\.(cue|toc)$/i.test(f.name))).map(r=><button key={`analyze-${r.id}`} onClick={()=>onOpenAnalysis(r.id,'files')}>Analyze retained digital audio · {r.files[0]?.name}</button>)}
  {canPrepareRequested&&item.digital_audio&&item.captures.map(capture=><PurchaseDetails key={`${item.id}/${capture.id}`} itemId={item.id} capture={capture} onChanged={()=>{setAvailabilityRefresh(v=>v+1);if(callbacks.current.onPrepared)void api<Item>(`/items/${item.id}`,{signal:requests.current.signal}).then(value=>callbacks.current.onPrepared?.(value)).catch(report);}}/>)}
  {rows.some(row=>row.album_selection==='needs_review')&&<p className="notice">Some tracks have more than one version. A curator can choose one with Edit track details before the album can play.</p>}{queueNotice&&<p role="status">{queueNotice}</p>}
  <div className="album-track-toolbar"><h3>Tracks</h3>{discs.length>0&&<button className="primary" disabled={loading||playBusy} onClick={()=>void play()}>Play album</button>}{!canPrepare&&discs.some(([,tracks])=>tracks.some(playableRow))&&<button onClick={()=>enqueue(discs.flatMap(([,tracks])=>tracks),true)}>Add album to queue</button>}<ActionMenu label="Album actions">{onAddToList&&discs.length>0&&<button onClick={()=>onAddToList()}>Add album to list</button>}<button disabled={busy} onClick={()=>{setError('');setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);}}>Refresh tracks</button></ActionMenu></div>
  {discs.map(([key,tracks],discIndex)=><article className="music-disc" key={key} data-disc-key={key}>
   <header><strong>{coverage?.sources.find(source=>`${source.capture_id}/${source.source_representation_id}`===key)?.medium_position?`Album disc ${coverage.sources.find(source=>`${source.capture_id}/${source.source_representation_id}`===key)!.medium_position}`:tracks[0][0].profile?`Disc ${discIndex+1}`:'Digital original'}</strong><button disabled={playBusy||!tracks.some(playableRow)} title="Replaces your queue" onClick={()=>void play(tracks[0][0].source_representation_id)}>Play original</button>{onAddToList&&<ActionMenu label={`Disc ${discIndex+1} actions`}><button onClick={()=>onAddToList(tracks[0][0].source_representation_id)}>{tracks[0][0].profile?'Add disc to list':'Add original to list'}</button></ActionMenu>}</header>
   <small className="playback-source">Original: {tracks[0][0].source_label} · Import {tracks[0][0].capture_id.slice(-6)}</small>
   {tracks.map((group,index)=>{const first=group[0];const playable=playableRow(group);const flac=group.find(row=>row.profile?.codec==='flac'&&row.state.status==='available');
    return <div className="music-track" key={trackKey(first)} data-track-position={first.track_position} data-representation-id={playable?.state.status==='available'?playable.state.representation_id:undefined}>
     <div className="track-title"><button className="track-play" disabled={playBusy||!playable} aria-label={`Play track ${first.display_position??first.track_position}: ${first.title}`} title={playable?'Play now (replaces your queue)':'Can’t play in this browser. See formats below.'} onClick={()=>void play(first.source_representation_id,first.track_position)}>Play</button><strong title={first.title}>{first.display_position??first.track_position}. {first.title}</strong>{!first.profile&&first.disc_position&&<small>Disc {first.disc_position}</small>}{(onAddToList||!canPrepare||canPrepareRequested)&&<ActionMenu label={`Track ${first.track_position} actions`}>{canPrepareRequested&&<button onClick={()=>setEditingTrack({source:first.source_representation_id,position:first.track_position,title:first.title,disc:first.disc_position?String(first.disc_position):'',number:String(first.display_position??first.track_position),albumSelected:first.album_selection==='included'})}>Edit track details</button>}{onAddToList&&<button onClick={()=>onAddToList(first.source_representation_id,first.track_position)}>Add track to list</button>}{!canPrepare&&<button disabled={!playable} onClick={()=>enqueue([group])}>Add to queue</button>}</ActionMenu>}</div>
     {editingTrack?.source===first.source_representation_id&&editingTrack.position===first.track_position&&<form aria-label="Edit purchased track" onSubmit={event=>{event.preventDefault();void saveTrack();}}><label><input type="checkbox" checked={editingTrack.albumSelected} onChange={e=>setEditingTrack({...editingTrack,albumSelected:e.target.checked})}/> Include in album playback and lists</label><label>Track title<input maxLength={4096} value={editingTrack.title} onChange={e=>setEditingTrack({...editingTrack,title:e.target.value})}/></label><label>Disc number<input type="number" min="1" max="65535" value={editingTrack.disc} onChange={e=>setEditingTrack({...editingTrack,disc:e.target.value})}/></label><label>Track number<input type="number" min="1" max="65535" value={editingTrack.number} onChange={e=>setEditingTrack({...editingTrack,number:e.target.value})}/></label><button disabled={busy}>Save track details</button><button type="button" disabled={busy} onClick={()=>setEditingTrack(undefined)}>Cancel track edit</button></form>}
     <div className="track-formats" role="group" aria-label="Track formats">{group.map(row=>{
      const label=row.profile?profileLabel(row.profile):`${row.audio_format?.codec.toUpperCase()||'Audio'} original`;
      const compactLabel=!row.profile?(row.audio_format?.codec.toUpperCase()||'Original'):row.profile.codec==='flac'?'FLAC':`${group.find(value=>value.profile?.codec==='opus')===row?'Opus ':''}${row.profile.bitrate_kbps}k`;
      const detail=`${label}: ${stateLabel(row)}`;
      if(row.state.status==='available'){const enriched=row.profile?.codec==='flac'&&item.domain.module==='org.junk.music';const url=enriched?`/api/v1/items/${item.id}/music-exports/${row.source_representation_id}/${row.track_position}`:mediaUrl(item.id,row)!;return <a className="track-format ready" key={label} href={url} download aria-label={enriched?'Download FLAC with album details and available cover art':`Download ${label} · ${stateLabel(row)}`} title={enriched?'Download FLAC with album details and available cover art':`Download ${detail}`}><span aria-hidden="true">✓</span> {compactLabel}</a>;}
      const pending=pendingTracks.has(`${trackKey(row)}/${label}`);
      if(canPrepare&&row.can_prepare&&row.state.status!=='preparing')return <button className="track-format prepare" key={label} disabled={pending} onClick={()=>void prepare(row)} aria-label={pending?`Submitting ${label}…`:`Prepare ${label}`} title={`Prepare ${detail}`}><span aria-hidden="true">{pending?'…':'+'}</span> {compactLabel}</button>;
      if(row.state.status==='preparing')return <span className="track-format preparing" key={label} role="img" aria-label={`${label}: Preparing…`} title={`${label}: Preparing…`}><span aria-hidden="true">…</span> {compactLabel}</span>;
      return <details className="track-format-details" key={label}><summary className="track-format unavailable" aria-label={detail} title={detail}><span aria-hidden="true">−</span> {compactLabel}</summary><p>{detail}</p></details>;
     })}{!canPrepare&&flac&&!canDecode(flac)&&<InfoHelp label="FLAC playback">This browser can’t play FLAC. Download the file to play it in another app.</InfoHelp>}</div>
    </div>;
   })}
  </article>)}
 </section>;
}
