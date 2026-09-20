import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';

type Availability=components['schemas']['MusicTrackAvailability'];
type AlbumPlan=components['schemas']['MusicAlbumPreparationPlan'];
type Batch=components['schemas']['MusicPreparationBatchView'];
type Item=components['schemas']['PresentedItem'];
type Props={item:Item;canPrepare:boolean;onSessionExpired:()=>void;onPrepared?:()=>void};

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

export default function MusicTracks({item,canPrepare,onSessionExpired,onPrepared}:Props){
 const [rows,setRows]=useState<Availability[]>([]);
 // Captures are independent records; publication need not revise the Item.
 const captureRevisions=item.captures.map(capture=>`${capture.id}:${capture.revision}`).join(',');
 const [loading,setLoading]=useState(true);
 const [error,setError]=useState('');
 const [playbackError,setPlaybackError]=useState('');
 const [busy,setBusy]=useState(false);
 const [pendingTracks,setPendingTracks]=useState<Set<string>>(new Set());
 const [plan,setPlan]=useState<AlbumPlan>();
 const [batches,setBatches]=useState<Batch[]>([]);
 const [discSources,setDiscSources]=useState<Record<string,string>>({});
 const [includedDiscs,setIncludedDiscs]=useState<Set<string>>(new Set());
 const [refresh,setRefresh]=useState(0);
 const [availabilityRefresh,setAvailabilityRefresh]=useState(0);
 const [activeDisc,setActiveDisc]=useState('');
 const [currentIndex,setCurrentIndex]=useState(0);
 const [loadedIdentity,setLoadedIdentity]=useState('');
 const [playbackState,setPlaybackState]=useState<'ready'|'loading'|'playing'|'paused'|'finished'>('ready');
 const [position,setPosition]=useState(0);
 const [duration,setDuration]=useState(0);
 const callbacks=useRef({onSessionExpired,onPrepared});callbacks.current={onSessionExpired,onPrepared};
 const requests=useRef(new AbortController());
 const commandKeys=useRef(new Map<string,string>());
 const initializedDiscs=useRef(new Set<string>());
 function commandKey(scope:string){const keys=commandKeys.current;const key=keys.get(scope)||crypto.randomUUID();keys.set(scope,key);return key;}
 const audio=useRef<HTMLAudioElement|null>(null);
 const playbackGeneration=useRef(0);
 const loaded=useRef('');
 const bindAudio=React.useCallback((element:HTMLAudioElement|null)=>{
  // React detaches refs before passive-effect cleanup. Release the old element
  // here so removed DOM nodes cannot retain playback or pending media requests.
  if(audio.current!==element){
   playbackGeneration.current++;
   const previous=audio.current;
   if(previous){previous.pause();previous.removeAttribute('src');previous.load();}
   loaded.current='';audio.current=element;
  }
 },[]);

 function stopAudio(){
  playbackGeneration.current++;loaded.current='';
  const element=audio.current;if(element){element.pause();element.removeAttribute('src');element.load();}
  setLoadedIdentity('');setPosition(0);setDuration(0);setPlaybackState('ready');
 }
 function report(value:unknown){
  if(isApiRequestError(value)&&value.code==='unauthenticated')callbacks.current.onSessionExpired();
  else setError(requestErrorMessage(value,'Access to this music is no longer available. Ask a collection owner to restore Viewer access.'));
 }
 useEffect(()=>{
  requests.current=new AbortController();commandKeys.current.clear();initializedDiscs.current.clear();
  setRows([]);setError('');setPlaybackError('');setPendingTracks(new Set());setPlan(undefined);setBatches([]);setDiscSources({});setIncludedDiscs(new Set());setBusy(false);setActiveDisc('');setCurrentIndex(0);stopAudio();
  return()=>{requests.current.abort();const element=audio.current;if(element){element.pause();element.removeAttribute('src');element.load();}};
 },[item.id]);

 // Availability is viewer-safe and resumes even when another user owns the Job.
 useEffect(()=>{
  const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;setLoading(true);
  async function load(){
   try{
    const next=await api<Availability[]>(`/items/${item.id}/music-tracks`,{signal:controller.signal});
    if(controller.signal.aborted)return;setRows(next);setLoading(false);
    timer=setTimeout(load,next.some(row=>row.state.status==='preparing')?1500:5000);
   }catch(value){if(!controller.signal.aborted){setLoading(false);if(isApiRequestError(value)&&(value.code==='forbidden'||value.code==='unauthenticated'||value.status===404)){setRows([]);stopAudio();}report(value);}}
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
   }catch(value){if(!controller.signal.aborted)report(value);}
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
 useEffect(()=>{
  if(!discs.length){setActiveDisc('');setCurrentIndex(0);return;}
  if(!discs.some(([key])=>key===activeDisc)){setActiveDisc(discs[0][0]);setCurrentIndex(0);stopAudio();}
 },[discs,activeDisc]);
 const queue=discs.find(([key])=>key===activeDisc)?.[1]||[];
 useEffect(()=>{if(currentIndex>=queue.length&&queue.length)setCurrentIndex(0);},[queue.length,currentIndex]);
 const selectedGroup=queue[currentIndex];
 const selected=selectedGroup&&preferredAvailable(selectedGroup);
 const selectedIdentity=selected?.state.status==='available'?selected.state.representation_id:'';
 useEffect(()=>{
  // A refreshed queue may lose/reorder a track or select a different profile.
  // Never keep playing bytes under a now-unrelated current-track label.
  if(loaded.current&&loaded.current!==selectedIdentity)stopAudio();
 },[rows,selectedIdentity]);

 async function selectTrack(key:string,index:number,play:boolean){
  const nextQueue=discs.find(([candidate])=>candidate===key)?.[1]||[];const next=nextQueue[index]&&preferredAvailable(nextQueue[index]);
  const generation=++playbackGeneration.current;
  setActiveDisc(key);setCurrentIndex(index);setPlaybackError('');
  if(!next){stopAudio();setPlaybackError('No prepared representation for this track is reported as decodable by this browser. Use a download link to open it in a compatible player.');return;}
  const url=mediaUrl(item.id,next);if(!url)return;const element=audio.current;if(!element)return;
  const identity=next.state.status==='available'?next.state.representation_id:'';
  if(loaded.current!==identity){loaded.current=identity;element.pause();element.src=url;element.load();setLoadedIdentity(identity);setPosition(0);setDuration(0);setPlaybackState('loading');}
  if(!play)element.pause();
  if(play){try{await element.play();}catch{if(generation!==playbackGeneration.current||audio.current!==element)return;setPlaybackState('paused');setPlaybackError('Playback did not begin. The browser may not support this format, or your session or collection access may have changed. Download the prepared file to open it in a compatible player.');}}
 }
 async function togglePlayback(){
  const element=audio.current;if(!element||!selected)return;
  if(!element.paused){playbackGeneration.current++;element.pause();return;}await selectTrack(activeDisc,currentIndex,true);
 }
 async function move(delta:number,automatic=false){
  const next=currentIndex+delta;
  if(next<0||next>=queue.length){if(automatic)setPlaybackState('finished');return;}
  await selectTrack(activeDisc,next,true);
 }
 function seek(value:number){
  const element=audio.current;if(!element||loadedIdentity!==selectedIdentity||!Number.isFinite(value))return;
  try{element.currentTime=Math.max(0,Math.min(value,duration));setPosition(element.currentTime);}catch{setPlaybackError('This browser could not seek in the prepared file. Download it to use a compatible player.');}
 }

 if(!discs.length&&!error&&item.domain.module!=='org.junk.music')return null;
 return <section className={canPrepare?'music-preparation':'music-player'} aria-label={canPrepare?'Music preparation':'Music player'}>
  <div className="music-heading">{item.primary_artwork?<img src={`/api/v1/collections/${item.collection_id}/assets/${item.primary_artwork}/thumbnail`} alt={`${item.title} cover`}/>:<div className="music-cover-placeholder" aria-label="No cover artwork">♪</div>}<div><h3>{canPrepare?'Music preparation':'Disc player'}</h3><strong>{item.title}</strong>{item.subtitle&&<small>{item.subtitle}</small>}</div></div>
  {loading&&<p role="status">Checking music availability…</p>}
  {!loading&&!discs.length&&!error&&<p className="notice">{item.captures.length?'No supported audio tracks were found in the attached captures. Use Music CD analysis in Edit / attach files to inspect the package layout and its limitations. Check that the complete BIN/CUE or supported redumper package was published. Original files remain available below.':'No dump is attached to this album yet. Use Edit / attach files to upload the complete dump package, review it, and publish it before preparing tracks.'}</p>}
  {canPrepare&&<p className="muted">Encoder setup is explained in Administration → Audio encoders. Preparation requires a configured server encoder.</p>}
  <p className="muted">{canPrepare?'Prepare a separate file for each track. FLAC preserves the audio exactly; Opus makes a smaller copy.':'Play prepared tracks in this browser, or download a copy to use another player.'}</p>
  {canPrepare&&plan&&<section className="album-preparation" aria-label="Prepare album"><div className="section-heading"><div><h4>Prepare album as FLAC</h4><p className="muted">One durable batch prepares separate validated files for every selected track. Closing this window does not cancel it.</p></div><span>{plan.total_tracks} tracks by default</span></div>
   {plan.discs.map((disc,index)=><fieldset key={disc.capture_id}><legend>Associated disc {index+1}</legend><label><input type="checkbox" checked={includedDiscs.has(disc.capture_id)} onChange={event=>setIncludedDiscs(current=>{const next=new Set(current);event.target.checked?next.add(disc.capture_id):next.delete(disc.capture_id);return next;})}/> Include this disc</label>{disc.sources.length===1?<p className="muted">{disc.sources[0].track_count} tracks · {disc.sources[0].already_available} ready · {disc.sources[0].preparing} preparing</p>:<>{disc.message&&<p className="notice">{disc.message}</p>}{disc.sources.map((source,sourceIndex)=><label key={source.source_representation_id}><input type="radio" name={`album-source-${disc.capture_id}`} checked={discSources[disc.capture_id]===source.source_representation_id} onChange={()=>setDiscSources(current=>({...current,[disc.capture_id]:source.source_representation_id}))}/> Source {sourceIndex+1}: {source.track_count} tracks · {source.already_available} ready</label>)}</>}</fieldset>)}
   {plan.blockers.map(message=><p className="notice" key={message}>{message}</p>)}
   <button className="primary" disabled={busy||!plan.can_submit||![...includedDiscs].some(capture=>discSources[capture])||plan.discs.some(disc=>includedDiscs.has(disc.capture_id)&&!discSources[disc.capture_id])} onClick={()=>void prepareAlbum()}>{busy?'Submitting…':'Prepare selected album'}</button>
  </section>}
  {canPrepare&&batches.length>0&&<section className="preparation-batches" aria-label="Album preparation progress"><h4>Preparation history</h4>{batches.map(batch=><article key={batch.batch_id}><div className="section-heading"><div><strong>FLAC album batch</strong><small>{new Date(batch.created_at).toLocaleString()} · {batch.batch_id}</small></div><span>{batch.succeeded}/{batch.members.length} ready</span></div><p className="muted">{batch.queued} queued · {batch.running} running · {batch.failed} failed · {batch.cancelled} cancelled · {batch.ineligible} ineligible{batch.cancelled_by_user?' · batch cancelled':''}</p>{batch.admission_error&&<p className="error" role="alert">Batch admission: {batch.admission_error}</p>}<details><summary>Track outcomes</summary>{batch.members.map(member=><p className="batch-member" key={member.member_id}><strong>{member.track_position}. {member.title}</strong><span className={`identity ${member.state}`}>{member.state.replaceAll('_',' ')}</span>{member.error&&<small>{member.error}</small>}</p>)}</details><div className="profile-actions">{batch.can_cancel&&<button disabled={busy} onClick={()=>void batchAction(batch,'cancel')}>Cancel outstanding tracks</button>}{batch.can_retry&&<button disabled={busy} onClick={()=>void batchAction(batch,'retry')}>Retry unfinished tracks</button>}</div></article>)}</section>}
  {!canPrepare&&<audio ref={bindAudio} preload="metadata" data-representation-id={loadedIdentity||undefined} onLoadedMetadata={event=>{setDuration(Number.isFinite(event.currentTarget.duration)?event.currentTarget.duration:0);setPlaybackState(event.currentTarget.paused?'paused':'playing');}} onDurationChange={event=>setDuration(Number.isFinite(event.currentTarget.duration)?event.currentTarget.duration:0)} onTimeUpdate={event=>setPosition(event.currentTarget.currentTime)} onPlay={event=>{if(!event.currentTarget.paused)setPlaybackState('playing');}} onPause={event=>{if(event.currentTarget.paused&&!event.currentTarget.ended&&event.currentTarget.currentSrc)setPlaybackState('paused');}} onEnded={event=>{if(event.currentTarget.ended)void move(1,true);}} onError={event=>{if(event.currentTarget.currentSrc){setPlaybackState('paused');setPlaybackError('Playback failed. The format may be unsupported, or your session or collection access may have changed. Refresh availability or download the prepared file to open it elsewhere.');}}}/>} 
  {discs.map(([key,tracks],discIndex)=><article className={`music-disc ${!canPrepare&&key===activeDisc?'active':''}`} key={key} data-disc-key={key}>
   <header><strong>Captured disc {discIndex+1}</strong>{!canPrepare&&discs.length>1&&<button onClick={()=>void selectTrack(key,0,false)}>{key===activeDisc?'Current disc':'Select disc'}</button>}</header>
   {tracks.map((group,index)=>{const first=group[0];const playable=preferredAvailable(group);const flac=group.find(row=>row.profile.codec==='flac'&&row.state.status==='available');const isCurrent=!canPrepare&&key===activeDisc&&index===currentIndex;
    return <div className={`music-track ${isCurrent?'current':''}`} key={trackKey(first)} data-track-position={first.track_position} data-representation-id={playable?.state.status==='available'?playable.state.representation_id:undefined}>
     <div>{canPrepare?<strong>{first.track_position}. {first.title}</strong>:<button className="track-select" aria-current={isCurrent?'true':undefined} onClick={()=>void selectTrack(key,index,true)}><strong>{first.track_position}. {first.title}</strong><small>{isCurrent?'Current track':playable?`${profileLabel(playable.profile)} ready`:'Download/open only'}</small></button>}</div>
     <div className="profile-actions">{group.map(row=>{
      if(row.state.status==='available'){const url=mediaUrl(item.id,row)!;return <a key={profileLabel(row.profile)} href={url} download>{profileLabel(row.profile)} <small>{stateLabel(row)}</small></a>;}
      const pending=pendingTracks.has(`${trackKey(row)}/${profileLabel(row.profile)}`);return canPrepare&&row.can_prepare?<button className={row.profile.codec==='flac'?'primary':''} key={profileLabel(row.profile)} disabled={pending} onClick={()=>void prepare(row)}>{pending?'Submitting…':`Prepare ${profileLabel(row.profile)}`}</button>:<span className="profile-status" key={profileLabel(row.profile)}><b>{profileLabel(row.profile)}</b><small>{stateLabel(row)}</small></span>;
     })}</div>
     {!canPrepare&&flac&&!canDecode(flac)&&<p className="codec-guidance">This browser does not report FLAC playback support. Download the FLAC file to open it in a compatible player.</p>}
    </div>;
   })}
  </article>)}
  {!canPrepare&&selectedGroup&&<div className="player-controls">
   <p className="now-playing" role="status"><span>{playbackState==='playing'?'Now playing':playbackState==='paused'?'Paused':playbackState==='loading'?'Loading':playbackState==='finished'?'Disc finished':'Ready'}</span><strong>{selectedGroup[0].track_position}. {selectedGroup[0].title}</strong>{selected&&<small>{profileLabel(selected.profile)}</small>}</p>
   <div className="transport"><button aria-label="Previous track" disabled={currentIndex===0} onClick={()=>void move(-1)}>Previous</button><button className="primary" aria-label={playbackState==='playing'?'Pause':'Play'} disabled={!selected} onClick={()=>void togglePlayback()}>{playbackState==='playing'?'Pause':'Play'}</button><button aria-label="Next track" disabled={currentIndex>=queue.length-1} onClick={()=>void move(1)}>Next</button></div>
   <label className="seek"><span>{Math.floor(position)}s</span><input aria-label={`Seek in ${selectedGroup[0].title}`} type="range" min="0" max={duration||0} step="0.05" value={Math.min(position,duration||0)} disabled={!loadedIdentity||duration<=0} onChange={event=>seek(Number(event.target.value))}/><span>{Math.floor(duration)}s</span></label>
  </div>}
  {error&&<p role="alert" className="error">{error}</p>}
  {playbackError&&<p role="alert" className="error">{playbackError}</p>}
  <button disabled={busy} onClick={()=>{setError('');setAvailabilityRefresh(value=>value+1);setRefresh(value=>value+1);}}>Refresh music availability</button>
 </section>;
}
