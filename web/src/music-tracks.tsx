import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';

type Availability=components['schemas']['MusicTrackAvailability'];
type Job=components['schemas']['JobView'];
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
 const [jobId,setJobId]=useState<string|null>(null);
 const [progress,setProgress]=useState('');
 const [refresh,setRefresh]=useState(0);
 const [activeDisc,setActiveDisc]=useState('');
 const [currentIndex,setCurrentIndex]=useState(0);
 const [loadedIdentity,setLoadedIdentity]=useState('');
 const [playbackState,setPlaybackState]=useState<'ready'|'loading'|'playing'|'paused'|'finished'>('ready');
 const [position,setPosition]=useState(0);
 const [duration,setDuration]=useState(0);
 const callbacks=useRef({onSessionExpired,onPrepared});callbacks.current={onSessionExpired,onPrepared};
 const request=useRef<AbortController|null>(null);
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
  setRows([]);setError('');setPlaybackError('');setJobId(null);setBusy(false);setProgress('');setActiveDisc('');setCurrentIndex(0);stopAudio();
  return()=>{request.current?.abort();const element=audio.current;if(element){element.pause();element.removeAttribute('src');element.load();}};
 },[item.id]);

 // Resume availability polling after reload/reopening, including another user's
 // active job. Detailed private progress is fetched only for our accepted job.
 useEffect(()=>{
  const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;setLoading(true);
  async function load(){
   try{
    const next=await api<Availability[]>(`/items/${item.id}/music-tracks`,{signal:controller.signal});
    if(controller.signal.aborted)return;setRows(next);setLoading(false);
    if(next.some(row=>row.state.status==='preparing'))timer=setTimeout(load,1500);
   }catch(value){if(!controller.signal.aborted){setLoading(false);if(isApiRequestError(value)&&(value.code==='forbidden'||value.code==='unauthenticated'||value.status===404)){setRows([]);stopAudio();}report(value);}}
  }
  void load();return()=>{controller.abort();clearTimeout(timer);};
 },[item.id,item.revision,captureRevisions,refresh]);

 useEffect(()=>{
  if(!jobId)return;const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;
  async function poll(){
   try{
    const job=await api<Job>(`/jobs/${jobId}`,{signal:controller.signal});if(controller.signal.aborted)return;
    if(job.state==='completed'||job.state==='failed'||job.state==='cancelled'){
     setJobId(null);setProgress('');setRefresh(value=>value+1);
     if(job.state==='completed')callbacks.current.onPrepared?.();else setError(job.error||`Preparation ${job.state}.`);return;
    }
    setProgress(job.total_units>0?`${job.phase} · ${job.completed_units}/${job.total_units}`:job.phase);timer=setTimeout(poll,1000);
   }catch(value){if(!controller.signal.aborted){report(value);setJobId(null);setProgress('');}}
  }
  void poll();return()=>{controller.abort();clearTimeout(timer);};
 },[jobId,item.id]);

 async function prepare(row:Availability){
  request.current?.abort();const controller=new AbortController();request.current=controller;setBusy(true);setError('');
  try{
   const accepted=await api<{job_id:string}>(`/items/${item.id}/captures/${row.capture_id}/music-tracks/${row.track_position}/prepare`,{method:'POST',signal:controller.signal,body:JSON.stringify({source_representation_id:row.source_representation_id,profile:row.profile})});
   if(controller.signal.aborted)return;setJobId(accepted.job_id);setProgress('Queued…');setRefresh(value=>value+1);
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
  <h3>{canPrepare?'Music preparation':'Disc player'}</h3>
  {loading&&<p role="status">Checking music availability…</p>}
  {!loading&&!discs.length&&!error&&<p className="notice">{item.captures.length?'No supported audio tracks were found in the attached captures. Use Music CD analysis in Edit / attach files to inspect the package layout and its limitations. Check that the complete BIN/CUE or supported redumper package was published. Original files remain available below.':'No dump is attached to this album yet. Use Edit / attach files to upload the complete dump package, review it, and publish it before preparing tracks.'}</p>}
  {canPrepare&&<p className="muted">Encoder setup is explained in Administration → Audio encoders. Preparation requires a configured server encoder.</p>}
  <p className="muted">{canPrepare?'Prepare a separate file for each track. FLAC preserves the audio exactly; Opus makes a smaller copy.':'Play prepared tracks in this browser, or download a copy to use another player.'}</p>
  {!canPrepare&&<audio ref={bindAudio} preload="metadata" data-representation-id={loadedIdentity||undefined} onLoadedMetadata={event=>{setDuration(Number.isFinite(event.currentTarget.duration)?event.currentTarget.duration:0);setPlaybackState(event.currentTarget.paused?'paused':'playing');}} onDurationChange={event=>setDuration(Number.isFinite(event.currentTarget.duration)?event.currentTarget.duration:0)} onTimeUpdate={event=>setPosition(event.currentTarget.currentTime)} onPlay={event=>{if(!event.currentTarget.paused)setPlaybackState('playing');}} onPause={event=>{if(event.currentTarget.paused&&!event.currentTarget.ended&&event.currentTarget.currentSrc)setPlaybackState('paused');}} onEnded={event=>{if(event.currentTarget.ended)void move(1,true);}} onError={event=>{if(event.currentTarget.currentSrc){setPlaybackState('paused');setPlaybackError('Playback failed. The format may be unsupported, or your session or collection access may have changed. Refresh availability or download the prepared file to open it elsewhere.');}}}/>} 
  {discs.map(([key,tracks],discIndex)=><article className={`music-disc ${!canPrepare&&key===activeDisc?'active':''}`} key={key} data-disc-key={key}>
   <header><strong>Captured disc {discIndex+1}</strong>{!canPrepare&&discs.length>1&&<button onClick={()=>void selectTrack(key,0,false)}>{key===activeDisc?'Current disc':'Select disc'}</button>}</header>
   {tracks.map((group,index)=>{const first=group[0];const playable=preferredAvailable(group);const flac=group.find(row=>row.profile.codec==='flac'&&row.state.status==='available');const isCurrent=!canPrepare&&key===activeDisc&&index===currentIndex;
    return <div className={`music-track ${isCurrent?'current':''}`} key={trackKey(first)} data-track-position={first.track_position} data-representation-id={playable?.state.status==='available'?playable.state.representation_id:undefined}>
     <div>{canPrepare?<strong>{first.track_position}. {first.title}</strong>:<button className="track-select" aria-current={isCurrent?'true':undefined} onClick={()=>void selectTrack(key,index,true)}><strong>{first.track_position}. {first.title}</strong><small>{isCurrent?'Current track':playable?`${profileLabel(playable.profile)} ready`:'Download/open only'}</small></button>}</div>
     <div className="profile-actions">{group.map(row=>{
      if(row.state.status==='available'){const url=mediaUrl(item.id,row)!;return <a key={profileLabel(row.profile)} href={url} download>{profileLabel(row.profile)} <small>{stateLabel(row)}</small></a>;}
      return canPrepare&&row.can_prepare?<button className={row.profile.codec==='flac'?'primary':''} key={profileLabel(row.profile)} disabled={busy||!!jobId} onClick={()=>void prepare(row)}>Prepare {profileLabel(row.profile)}</button>:<span className="profile-status" key={profileLabel(row.profile)}><b>{profileLabel(row.profile)}</b><small>{stateLabel(row)}</small></span>;
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
  {progress&&<p role="status">Preparation: {progress}</p>}
  {error&&<p role="alert" className="error">{error}</p>}
  {playbackError&&<p role="alert" className="error">{playbackError}</p>}
  <button disabled={busy} onClick={()=>{setError('');setRefresh(value=>value+1);}}>Refresh music availability</button>
 </section>;
}
