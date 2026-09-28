import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage,addMusicTracks} from '@collector/runtime';
import type {components} from './generated/api';

type Availability=components['schemas']['MusicTrackAvailability'];
type AlbumPlan=components['schemas']['MusicAlbumPreparationPlan'];
type Batch=components['schemas']['MusicPreparationBatchView'];
type Item=components['schemas']['PresentedItem'];
type Props={item:Item;canPrepare:boolean;onSessionExpired:()=>void;onPrepared?:(item:Item)=>void;onOpenHistory?:()=>void};

function profileLabel(profile:Availability['profile']):string{
 return profile.codec==='flac'?'FLAC':`Opus ${profile.bitrate_kbps} kbps`;
}
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
 return available.find(row=>row.profile.codec==='flac')||available[0];
}
function discKey(row:Availability):string{return `${row.capture_id}/${row.source_representation_id}`;}
function trackKey(row:Availability):string{return `${discKey(row)}/${row.track_position}`;}

export default function MusicTracks({item,canPrepare,onSessionExpired,onPrepared,onOpenHistory}:Props){
 const [rows,setRows]=useState<Availability[]>([]);
 // Captures are independent records; publication need not revise the Item.
 const captureRevisions=item.captures.map(capture=>`${capture.id}:${capture.revision}`).join(',');
 const [loading,setLoading]=useState(true);
 const [error,setError]=useState('');
 const [queueNotice,setQueueNotice]=useState('');
 const [busy,setBusy]=useState(false);
 const [pendingTracks,setPendingTracks]=useState<Set<string>>(new Set());
 const [plan,setPlan]=useState<AlbumPlan>();
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
  else setError(requestErrorMessage(value,'Access to this music is no longer available. Ask a collection owner to restore Viewer access.'));
 }
 useEffect(()=>{
  requests.current=new AbortController();commandKeys.current.clear();initializedDiscs.current.clear();notifiedOutputs.current=undefined;
  setRows([]);setError('');setQueueNotice('');setPendingTracks(new Set());setPlan(undefined);setBatches([]);setDiscSources({});setIncludedDiscs(new Set());setBusy(false);
  return()=>{requests.current.abort();};
 },[item.id]);

 // Availability is viewer-safe and resumes even when another user owns the Job.
 useEffect(()=>{
  const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;setLoading(true);
  async function load(){
   try{
    const next=await api<Availability[]>(`/items/${item.id}/music-tracks`,{signal:controller.signal});
    if(controller.signal.aborted)return;setRows(next);setLoading(false);
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
     api<AlbumPlan>(`/items/${item.id}/music-preparation-plan`,{signal:controller.signal}),
     api<Batch[]>(`/items/${item.id}/music-preparation-batches`,{signal:controller.signal}),
    ]);if(controller.signal.aborted)return;
    setPlan(nextPlan);setBatches(nextBatches);
    setDiscSources(current=>{const next={...current};for(const disc of nextPlan.discs){const selected=disc.sources.find(source=>source.default_selected);if(selected&&!next[disc.capture_id])next[disc.capture_id]=selected.source_representation_id;}return next;});
    const added=nextPlan.discs.filter(disc=>!initializedDiscs.current.has(disc.capture_id)).map(disc=>disc.capture_id);
    for(const id of added)initializedDiscs.current.add(id);
    if(added.length)setIncludedDiscs(current=>new Set([...current,...added]));
    if(nextBatches.some(batch=>batch.queued>0||batch.running>0))timer=setTimeout(load,1200);
   }catch(value){if(!controller.signal.aborted){report(value);if(!isApiRequestError(value)||(!['forbidden','unauthenticated'].includes(value.code)&&value.status!==404))timer=setTimeout(load,5000);}}
  }
  void load();return()=>{controller.abort();clearTimeout(timer);};
 },[canPrepare,item.id,item.revision,captureRevisions,refresh]);

 async function prepare(row:Availability){
  const controller=requests.current;const key=`${trackKey(row)}/${profileLabel(row.profile)}`;setPendingTracks(current=>new Set(current).add(key));setError('');
  try{
   await api<{job_id:string}>(`/items/${item.id}/captures/${row.capture_id}/music-tracks/${row.track_position}/prepare`,{method:'POST',signal:controller.signal,headers:{'Idempotency-Key':commandKey(key)},body:JSON.stringify({source_representation_id:row.source_representation_id,profile:row.profile})});
   if(controller.signal.aborted)return;commandKeys.current.delete(key);
   setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);
  }catch(value){if(!controller.signal.aborted)report(value);}finally{if(!controller.signal.aborted)setPendingTracks(current=>{const next=new Set(current);next.delete(key);return next;});}
 }

 async function prepareAlbum(){
  if(!plan)return;const sources=plan.discs.filter(disc=>includedDiscs.has(disc.capture_id)).map(disc=>discSources[disc.capture_id]).filter(Boolean);
  const controller=requests.current;const key=`album:${JSON.stringify([...sources].sort())}`;
  setBusy(true);setError('');try{
   const batch=await api<Batch>(`/items/${item.id}/music-preparation-batches`,{method:'POST',signal:controller.signal,headers:{'Idempotency-Key':commandKey(key)},body:JSON.stringify({source_representation_ids:sources})});
   if(controller.signal.aborted)return;commandKeys.current.delete(key);
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

 const discs=React.useMemo(()=>{
  const grouped=new Map<string,Map<number,Availability[]>>();
  for(const row of rows){const key=discKey(row);const tracks=grouped.get(key)||new Map<number,Availability[]>();const variants=tracks.get(row.track_position)||[];variants.push(row);tracks.set(row.track_position,variants);grouped.set(key,tracks);}
  return [...grouped.entries()].map(([key,tracks])=>[key,[...tracks.values()]] as const);
 },[rows]);
 function enqueue(groups:Availability[][]){
  const tracks=groups.flatMap(group=>{const row=preferredAvailable(group);return row?.state.status==='available'?[{itemId:item.id,collectionId:item.collection_id,album:item.title,title:row.title,position:row.track_position,representationId:row.state.representation_id,fileIndex:row.state.file_index,mediaType:row.state.media_type,artwork:item.primary_artwork||undefined}]:[];});
  const added=addMusicTracks(tracks);setQueueNotice(added===tracks.length?`Added ${added} ${added===1?'track':'tracks'} to your queue.`:`Added ${added} tracks. Your queue holds up to 500 tracks; remove some to add more.`);
 }

 if(!discs.length&&!error&&item.domain.module!=='org.junk.music')return null;
 return <section className={canPrepare?'music-preparation':'album-tracks'} aria-label={canPrepare?'Music preparation':'Album tracks'}>
  <div className="music-heading">{item.primary_artwork?<img src={`/api/v1/collections/${item.collection_id}/assets/${item.primary_artwork}/thumbnail`} alt={`${item.title} cover`}/>:<div className="music-cover-placeholder" aria-label="No cover artwork">♪</div>}<div><h3>{canPrepare?'Music preparation':'Album tracks'}</h3><strong>{item.title}</strong>{item.subtitle&&<small>{item.subtitle}</small>}</div></div>
  {loading&&<p role="status">Checking music availability…</p>}
  {!loading&&!discs.length&&!error&&<p className="notice">{item.captures.length?(item.content_permissions.can_edit?'No supported audio tracks were found. Open Edit / attach files → Advanced evidence to inspect the CD layout and limitations. Check that the complete package was published. Originals remain available below.':'No supported audio tracks were found. Ask a collection curator to inspect the CD layout and complete package. Originals remain available below.'):(item.content_permissions.can_edit?'No dump is attached yet. Open Edit / attach files → Files to preserve a complete package.':'No dump is attached yet. Ask a collection curator to attach a complete package.')}</p>}
  {canPrepare&&<p className="muted">Encoder setup is explained in Administration → Audio encoders. Preparation requires a configured server encoder.</p>}
  <p className="muted">{canPrepare?'Prepared tracks are saved automatically; Save inventory is not required. FLAC preserves the audio exactly; Opus makes a smaller copy.':'Add prepared tracks to your queue below, or download a copy. Browsing another album keeps your music playing.'}</p>
  {error&&<p role="alert" className="error">{error}</p>}
  {canPrepare&&plan&&<section className="album-preparation" aria-label="Prepare album"><div className="section-heading"><div><h4>Prepare album as FLAC</h4><p className="muted">One durable batch prepares separate validated files for every selected track. Closing this window does not cancel it.</p></div><span>{plan.total_tracks} tracks by default</span></div>
   {plan.discs.map((disc,index)=><fieldset key={disc.capture_id}><legend>Associated disc {index+1}</legend><label><input type="checkbox" checked={includedDiscs.has(disc.capture_id)} onChange={event=>setIncludedDiscs(current=>{const next=new Set(current);event.target.checked?next.add(disc.capture_id):next.delete(disc.capture_id);return next;})}/> Include this disc</label>{disc.sources.length===1?<p className="muted">{disc.sources[0].track_count} tracks · {disc.sources[0].already_available} ready · {disc.sources[0].preparing} preparing</p>:<>{disc.message&&<p className="notice">{disc.message}</p>}{disc.sources.map((source,sourceIndex)=><label key={source.source_representation_id}><input type="radio" name={`album-source-${disc.capture_id}`} checked={discSources[disc.capture_id]===source.source_representation_id} onChange={()=>setDiscSources(current=>({...current,[disc.capture_id]:source.source_representation_id}))}/> Source {sourceIndex+1}: {source.track_count} tracks · {source.already_available} ready</label>)}</>}</fieldset>)}
   {plan.blockers.map(message=><p className="notice" key={message}>{message}</p>)}
   <button className="primary" disabled={busy||!plan.can_submit||![...includedDiscs].some(capture=>discSources[capture])||plan.discs.some(disc=>includedDiscs.has(disc.capture_id)&&!discSources[disc.capture_id])} onClick={()=>void prepareAlbum()}>{busy?'Submitting…':'Prepare selected album'}</button>
  </section>}
  {canPrepare&&batches.length>0&&<section className="preparation-batches" aria-label="Album preparation progress"><h4>Preparation history</h4>{batches.length===50&&<p className="muted">Showing the 50 most recent batches. Older track jobs are in this entry’s Work &amp; history. {onOpenHistory&&<button onClick={onOpenHistory}>Open older preparation work</button>}</p>}{batches.map(batch=><article key={batch.batch_id}><div className="section-heading"><div><strong>FLAC album batch</strong><small>{new Date(batch.created_at).toLocaleString()} · {batch.batch_id}</small></div><span>{batch.succeeded}/{batch.members.length} ready</span></div><p className="muted">{batch.queued} queued · {batch.running} running · {batch.failed} failed · {batch.cancelled} cancelled · {batch.ineligible} ineligible{batch.cancelled_by_user?' · batch cancelled':''}</p>{batch.admission_error&&<p className="error" role="alert">Batch admission: {batch.admission_error}</p>}<details><summary>Track outcomes</summary>{batch.members.map(member=><p className="batch-member" key={member.member_id}><strong>{member.track_position}. {member.title}</strong><span className={`identity ${member.state}`}>{member.state.replaceAll('_',' ')}</span>{member.error&&<small>{member.error}</small>}</p>)}</details><div className="profile-actions">{batch.can_cancel&&<button disabled={busy} onClick={()=>void batchAction(batch,'cancel')}>Cancel outstanding tracks</button>}{batch.can_retry&&<button disabled={busy} onClick={()=>void batchAction(batch,'retry')}>Retry unfinished tracks</button>}</div></article>)}</section>}
  {queueNotice&&<p role="status">{queueNotice}</p>}
  {!canPrepare&&discs.some(([,tracks])=>tracks.some(preferredAvailable))&&<button onClick={()=>enqueue(discs.flatMap(([,tracks])=>tracks))}>Add album to queue</button>}
  {discs.map(([key,tracks],discIndex)=><article className="music-disc" key={key} data-disc-key={key}>
   <header><strong>Captured disc {discIndex+1}</strong></header>
   {tracks.map((group,index)=>{const first=group[0];const playable=preferredAvailable(group);const flac=group.find(row=>row.profile.codec==='flac'&&row.state.status==='available');
    return <div className="music-track" key={trackKey(first)} data-track-position={first.track_position} data-representation-id={playable?.state.status==='available'?playable.state.representation_id:undefined}>
     <div><strong>{first.track_position}. {first.title}</strong>{!canPrepare&&<button disabled={!playable} onClick={()=>enqueue([group])}>Add to queue</button>}</div>
     <div className="profile-actions">{group.map(row=>{
      if(row.state.status==='available'){const url=mediaUrl(item.id,row)!;return <a key={profileLabel(row.profile)} href={url} download>{profileLabel(row.profile)} <small>{stateLabel(row)}</small></a>;}
      const pending=pendingTracks.has(`${trackKey(row)}/${profileLabel(row.profile)}`);return canPrepare&&row.can_prepare?<button className={row.profile.codec==='flac'?'primary':''} key={profileLabel(row.profile)} disabled={pending} onClick={()=>void prepare(row)}>{pending?'Submitting…':`Prepare ${profileLabel(row.profile)}`}</button>:<span className="profile-status" key={profileLabel(row.profile)}><b>{profileLabel(row.profile)}</b><small>{stateLabel(row)}</small></span>;
     })}</div>
     {!canPrepare&&flac&&!canDecode(flac)&&<p className="codec-guidance">This browser does not report FLAC playback support. Download the FLAC file to open it in a compatible player.</p>}
    </div>;
   })}
  </article>)}
  <button disabled={busy} onClick={()=>{setError('');setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);}}>Refresh music availability</button>
 </section>;
}
