import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from './runtime';
import {useMusicQueue,selectMusicTrack,removeMusicTrack,reorderMusicTrack,clearMusicQueue,musicTrackUrl,type QueuedTrack} from './music-queue';
import type {components} from './generated/api';
type Availability=components['schemas']['MusicTrackAvailability'];
const time=(n:number)=>`${Math.floor(n/60)}:${String(Math.floor(n%60)).padStart(2,'0')}`;

export default function MusicPlayer({browsingMusic,onSessionExpired}:{browsingMusic:boolean;onSessionExpired:()=>void}){
 const queue=useMusicQueue();const current=queue.tracks.find(t=>t.id===queue.current);const index=queue.tracks.findIndex(t=>t.id===queue.current);
 const [expanded,setExpanded]=useState(false);const [error,setError]=useState('');const [state,setState]=useState('Ready');const [position,setPosition]=useState(0);const [duration,setDuration]=useState(0);
 const audio=useRef<HTMLAudioElement|null>(null);const dock=useRef<HTMLElement>(null);const queueButton=useRef<HTMLButtonElement>(null);const generation=useRef(0);const active=useRef<string|null>(null);const loaded=useRef('');const request=useRef<AbortController|undefined>(undefined);
 const expired=useRef(onSessionExpired);expired.current=onSessionExpired;
 const bindAudio=React.useCallback((element:HTMLAudioElement|null)=>{if(audio.current!==element){generation.current++;request.current?.abort();const previous=audio.current;if(previous){previous.pause();previous.removeAttribute('src');previous.load();}audio.current=element;loaded.current='';active.current=null;}},[]);
 function release(){generation.current++;request.current?.abort();active.current=null;loaded.current='';const element=audio.current;if(element){element.pause();element.removeAttribute('src');element.load();}setPosition(0);setDuration(0);setState('Ready');}
 useEffect(()=>{if(active.current!==current?.id){release();setError('');}},[current?.id]);
 const visible=browsingMusic||queue.tracks.length>0;
 useEffect(()=>{if(!visible)return;const element=dock.current;if(!element)return;const observer=new ResizeObserver(()=>document.documentElement.style.setProperty('--music-dock-height',`${element.getBoundingClientRect().height}px`));observer.observe(element);document.body.classList.add('has-music-dock');return()=>{observer.disconnect();document.body.classList.remove('has-music-dock');document.documentElement.style.removeProperty('--music-dock-height');};},[visible]);
 function available(rows:Availability[],track:QueuedTrack){return rows.some(row=>row.state.status==='available'&&row.state.representation_id===track.representationId&&row.state.file_index===track.fileIndex);}
 async function checkTrack(track:QueuedTrack,signal:AbortSignal){
  if(track.original){const item=await api<components['schemas']['PresentedItem']>(`/items/${track.itemId}`,{signal});return !item.deleted&&item.captures.some(c=>c.representations.some(r=>r.id===track.representationId&&r.role==='preservation_master'&&!!r.files[track.fileIndex]));}
  return available(await api<Availability[]>(`/items/${track.itemId}/music-tracks`,{signal}),track);
 }
 // Revalidate independently of the album view: buffered media must stop on loss
 // of access even while browsing another collection. Transient outages can retry.
 useEffect(()=>{if(!current)return;const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;
 async function check(){try{const allowed=await checkTrack(current!,controller.signal);if(!controller.signal.aborted&&!allowed){release();setError('This queued file is no longer available. Remove it or choose another track.');}}catch(e){if(controller.signal.aborted)return;if(isApiRequestError(e)&&[401,403,404].includes(e.status)){release();setError('Access to this queued album is no longer available.');if(e.status===401)expired.current();}else setError(`Could not check music availability. ${requestErrorMessage(e)} Use Play to retry.`);}finally{if(!controller.signal.aborted)timer=setTimeout(check,5000);}}
 void check();return()=>{controller.abort();clearTimeout(timer);};},[current?.id]);
 async function play(track:QueuedTrack){
  const element=audio.current;if(!element)return;request.current?.abort();const controller=new AbortController();request.current=controller;const token=++generation.current;
  const changed=active.current!==track.id;active.current=track.id;selectMusicTrack(track.id);setError('');setState('Loading');
  if(changed){element.pause();element.removeAttribute('src');element.load();loaded.current='';setPosition(0);setDuration(0);}
  try{const allowed=await checkTrack(track,controller.signal);if(token!==generation.current||audio.current!==element)return;
   if(!allowed)throw new Error('This queued file is no longer available. Remove it or choose another track.');
   if(!element.canPlayType(track.mediaType))throw new Error('This browser cannot play this format. Download the track from the album to use another player.');
   const url=musicTrackUrl(track);if(loaded.current!==url){element.src=url;element.load();loaded.current=url;}
   await element.play();
  }catch(e){if(controller.signal.aborted||token!==generation.current||audio.current!==element)return;element.pause();setState('Paused');setError(requestErrorMessage(e));if(isApiRequestError(e)&&e.status===401)expired.current();}
 }
 function toggle(){const element=audio.current;if(!element||!current)return;if(!element.paused||state==='Loading'){generation.current++;request.current?.abort();element.pause();setState('Paused');}else void play(current);}
 function move(delta:number,automatic=false){const track=queue.tracks[index+delta];if(track)void play(track);else if(automatic)setState('Queue finished');}
 if(!visible)return null;
 return <section ref={dock} className="music-player music-dock" aria-label="Music player">
  <audio ref={bindAudio} preload="none" data-representation-id={current?.representationId} onLoadedMetadata={e=>setDuration(Number.isFinite(e.currentTarget.duration)?e.currentTarget.duration:0)} onDurationChange={e=>setDuration(Number.isFinite(e.currentTarget.duration)?e.currentTarget.duration:0)} onTimeUpdate={e=>setPosition(e.currentTarget.currentTime)} onPlay={e=>{if(!e.currentTarget.paused)setState('Now playing');}} onPause={e=>{if(e.currentTarget.paused&&!e.currentTarget.ended&&e.currentTarget.currentSrc)setState('Paused');}} onEnded={e=>{if(e.currentTarget.ended)move(1,true);}} onError={e=>{if(e.currentTarget.error&&e.currentTarget.currentSrc){setState('Paused');setError('Playback failed. Use Play to retry, or download the track from its album.');}}}/>
  {expanded&&<section className="music-queue" aria-label="Currently playing list" onKeyDown={e=>{if(e.key==='Escape'){e.stopPropagation();setExpanded(false);queueButton.current?.focus();}}}><header><div><h3>Currently playing</h3><small>Saved in this browser for your user. Up to 500 tracks.</small></div><button onClick={()=>{clearMusicQueue();setExpanded(false);queueButton.current?.focus();}}>Clear queue</button></header>{!queue.tracks.length?<p>Add tracks from an album’s info view.</p>:<ol>{queue.tracks.map((track,i)=><li key={track.id} aria-current={track.id===queue.current?'true':undefined}><button className="queue-track" onClick={()=>void play(track)}><strong>{track.position}. {track.title}</strong><small>{track.album}</small></button><div className="queue-actions"><button aria-label={`Move ${track.title} up`} disabled={i===0} onClick={()=>reorderMusicTrack(track.id,-1)}>↑</button><button aria-label={`Move ${track.title} down`} disabled={i===queue.tracks.length-1} onClick={()=>reorderMusicTrack(track.id,1)}>↓</button><button aria-label={`Remove ${track.title} from queue`} onClick={()=>{removeMusicTrack(track.id);requestAnimationFrame(()=>{const buttons=dock.current?.querySelectorAll<HTMLButtonElement>('.queue-track');(buttons?.[Math.min(i,(buttons?.length||1)-1)]||queueButton.current)?.focus();});}}>Remove</button></div></li>)}</ol>}</section>}
  <div className="dock-main"><div className="dock-track">{current?.artwork&&<img src={`/api/v1/collections/${current.collectionId}/assets/${current.artwork}/thumbnail`} alt=""/>}<p className="now-playing" role="status"><span>{current?state:'Your music queue'}</span><strong>{current?`${current.position}. ${current.title}`:'Add tracks from an album'}</strong>{current&&<small>{current.album}</small>}</p></div>
   <div><div className="transport"><button aria-label="Previous track" disabled={index<=0} onClick={()=>move(-1)}>Previous</button><button className="primary" aria-label={state==='Now playing'||state==='Loading'?'Pause':'Play'} disabled={!current} onClick={toggle}>{state==='Now playing'||state==='Loading'?'Pause':'Play'}</button><button aria-label="Next track" disabled={index<0||index>=queue.tracks.length-1} onClick={()=>move(1)}>Next</button></div><label className="seek"><span>{time(position)}</span><input aria-label={`Seek in ${current?.title||'track'}`} type="range" min="0" max={duration||0} step="0.05" value={Math.min(position,duration||0)} disabled={!loaded.current||duration<=0} onChange={e=>{if(audio.current){try{audio.current.currentTime=Number(e.target.value);setPosition(audio.current.currentTime);}catch{setError('This browser could not seek in the file.');}}}}/><span>{time(duration)}</span></label></div>
   <button ref={queueButton} className="queue-toggle" aria-expanded={expanded} onClick={()=>setExpanded(v=>!v)}>Queue ({queue.tracks.length})</button>
  </div>{error&&<p role="alert" className="error dock-error">{error}</p>}
 </section>;
}
