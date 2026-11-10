import PlaybackFormatControl from './playback-format';
import {getPlaybackPreference} from './playback-preference';
import React,{useEffect,useRef,useState,ApiRequestError,isApiRequestError,requestErrorMessage} from './runtime';
import {useMusicQueue,selectMusicTrack,removeMusicTrack,reorderMusicTrack,clearMusicQueue,musicTrackUrl,streamTrackUrl,type QueuedTrack} from './music-queue';
import {prepareLocalPlayback,checkPlaybackAccess,playbackUrl,remotePlaybackExpired,remotePlaybackActive,normalPlaybackAvailable} from './local-playback';
import {useMusicMediaSession,clearMusicMediaSession} from './music-media-session';
const time=(n:number)=>`${Math.floor(n/60)}:${String(Math.floor(n%60)).padStart(2,'0')}`;

export default function MusicPlayer({browsingMusic,onSessionExpired}:{browsingMusic:boolean;onSessionExpired:()=>void}){
 const queue=useMusicQueue();const current=queue.tracks.find(t=>t.id===queue.current);const index=queue.tracks.findIndex(t=>t.id===queue.current);
 const [routeLabel,setRouteLabel]=useState('');
 const live=useRef<{offset:number;duration:number}|undefined>(undefined);
 const liveSeek=useRef<ReturnType<typeof setTimeout>|undefined>(undefined);
 const resume=useRef<number|undefined>(undefined);
 const [expanded,setExpanded]=useState(false);const [error,setError]=useState('');const [state,setState]=useState('Ready');const [position,setPosition]=useState(0);const [duration,setDuration]=useState(0);
 const audio=useRef<HTMLAudioElement|null>(null);const dock=useRef<HTMLElement>(null);const queueButton=useRef<HTMLButtonElement>(null);const generation=useRef(0);const active=useRef<string|null>(null);const loaded=useRef('');const request=useRef<AbortController|undefined>(undefined);const diagnosis=useRef<AbortController|undefined>(undefined);
 const recoveredGrant=useRef(false);
 const streamCheck=useRef<{track:string;path?:string}|undefined>(undefined);
 const pendingSeek=useRef<{element:HTMLAudioElement;position:number}|undefined>(undefined);
 const expired=useRef(onSessionExpired);expired.current=onSessionExpired;
 const bindAudio=React.useCallback((element:HTMLAudioElement|null)=>{if(audio.current!==element){clearTimeout(liveSeek.current);liveSeek.current=undefined;live.current=undefined;resume.current=undefined;streamCheck.current=undefined;generation.current++;request.current?.abort();diagnosis.current?.abort();const previous=audio.current;if(previous){clearMusicMediaSession();previous.pause();previous.removeAttribute('src');previous.load();}audio.current=element;loaded.current='';active.current=null;}},[]);
 function release(){clearTimeout(liveSeek.current);liveSeek.current=undefined;live.current=undefined;resume.current=undefined;streamCheck.current=undefined;clearMusicMediaSession();pendingSeek.current=undefined;generation.current++;request.current?.abort();diagnosis.current?.abort();active.current=null;loaded.current='';const element=audio.current;if(element){element.pause();element.removeAttribute('src');element.load();}setPosition(0);setDuration(0);setState('Ready');}
 useEffect(()=>{if(active.current!==current?.id){release();setError('');}},[current?.id]);
 const visible=browsingMusic||queue.tracks.length>0;
 useEffect(()=>{if(!visible)return;const element=dock.current;if(!element)return;const observer=new ResizeObserver(()=>document.documentElement.style.setProperty('--music-dock-height',`${element.getBoundingClientRect().height}px`));observer.observe(element);document.body.classList.add('has-music-dock');return()=>{observer.disconnect();document.body.classList.remove('has-music-dock');document.documentElement.style.removeProperty('--music-dock-height');};},[visible]);
 // The queue holds immutable files, including retained historical captures.
 async function checkTrack(track:QueuedTrack,signal:AbortSignal,renew=false){
  await prepareLocalPlayback(signal);
  let source=await playbackUrl(musicTrackUrl(track),signal,renew);
  const head=(url:string)=>fetch(url,{method:'HEAD',credentials:new URL(url,location.href).origin===location.origin?'same-origin':'omit',redirect:'error',signal,cache:'no-store'});
  let response=await head(source);
  if(response.status===401&&source!==musicTrackUrl(track)){source=await playbackUrl(musicTrackUrl(track),signal,true);response=await head(source);}
  if(!response.ok)throw new ApiRequestError(response.status,`http_${response.status}`,'The queued file could not be accessed.');
  return true;
 }
 async function diagnosePlaybackFailure(track:QueuedTrack,element:HTMLAudioElement,token:number){
  diagnosis.current?.abort();const controller=new AbortController();diagnosis.current=controller;
  try{const source=await playbackUrl(musicTrackUrl(track),controller.signal);const response=await fetch(source,{headers:{Range:'bytes=0-0'},credentials:new URL(source,location.href).origin===location.origin?'same-origin':'omit',redirect:'error',signal:controller.signal,cache:'no-store'});
   if(controller.signal.aborted||token!==generation.current||audio.current!==element||active.current!==track.id)return;
   if(response.status===409){const body:unknown=await response.json();if(controller.signal.aborted||token!==generation.current||audio.current!==element||active.current!==track.id)return;
    if(body&&typeof body==='object'&&'error' in body){const detail=(body as {error?:{code?:string}}).error;if(detail?.code==='prepared_media_invalid'){setError('This prepared audio file is missing, unreadable, or has changed. The original is saved; ask a curator to repair this prepared track.');return;}}}
   if([401,403,404].includes(response.status)){release();setError(response.status===404?'This queued audio file is missing or unavailable. Choose another track or ask a curator to repair it.':'Access to this queued album is no longer available.');if(response.status===401)expired.current();}
  }catch{ /* Keep the playback error when diagnosis is unavailable. */ }
 }
 // Revalidate independently of the album view: buffered media must stop on loss
 // of access even while browsing another collection. Transient outages can retry.
 useEffect(()=>{if(!current)return;const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;
 async function check(){
  const track=current!;
  const selected=streamCheck.current;
  const token=generation.current;
  try{
   const path=selected?.track===track.id?selected.path:track.liveOnly?undefined:musicTrackUrl(track);
   if(path)await checkPlaybackAccess(path,controller.signal);
  }catch(e){
   if(controller.signal.aborted||token!==generation.current||selected!==streamCheck.current)return;
   if(isApiRequestError(e)&&[401,403,404].includes(e.status)){release();setError('Access to this queued album is no longer available.');if(e.status===401)expired.current();}
   else setError(`Could not check music availability. ${requestErrorMessage(e)} Use Play to retry.`);
  }finally{if(!controller.signal.aborted)timer=setTimeout(check,5000);}
 }
 void check();return()=>{controller.abort();clearTimeout(timer);};},[current?.id]);
 async function play(track:QueuedTrack,renew=false,seek?:number){
  clearTimeout(liveSeek.current);liveSeek.current=undefined;
  if(!renew)recoveredGrant.current=false;
  const element=audio.current;if(!element)return;request.current?.abort();diagnosis.current?.abort();const controller=new AbortController();request.current=controller;const token=++generation.current;
  const changed=active.current!==track.id;active.current=track.id;selectMusicTrack(track.id);setError('');setState('Loading');
  const previousPosition=changed||element.ended?0:seek??resume.current??(live.current?.offset||0)+element.currentTime;
  const preference=getPlaybackPreference();
  const requested=preference.format;
  resume.current=undefined;
  if(changed||requested!=='saved'||live.current){live.current=undefined;pendingSeek.current=undefined;element.pause();element.removeAttribute('src');element.load();loaded.current='';if(changed){setPosition(0);setDuration(0);}}
  try{
   const route=await prepareLocalPlayback(controller.signal);
   if(token!==generation.current||audio.current!==element)return;
   const preferred=requested==='flac'?'audio/flac':'audio/ogg; codecs=opus';
   let url:string, liveOffset:number|undefined, checkPath:string|undefined;
   if(requested!=='saved'&&normalPlaybackAvailable()){
    streamCheck.current={track:track.id};
    if(!element.canPlayType(preferred))throw new Error('This browser cannot play the selected format. Choose another playback format.');
    url=streamTrackUrl(track,requested,preference.bitrate);
    const head=await fetch(url,{method:'HEAD',credentials:'same-origin',redirect:'error',signal:controller.signal,cache:'no-store'});
    if(!head.ok)throw new ApiRequestError(head.status,`http_${head.status}`,head.status===429?'Live playback is busy. Use Play to retry.':'The requested audio could not be opened. Try another track or choose Saved audio.');
    checkPath=head.headers.get('x-collector-playback-check')||undefined;
    if(head.headers.get('accept-ranges')!=='bytes'){
     liveOffset=Math.min(Math.max(0,previousPosition),Math.max(0,(track.seconds??previousPosition+1)-0.05));
     url=streamTrackUrl(track,requested,preference.bitrate,liveOffset);
    }
   }else{
    if(track.liveOnly)throw new Error('This track needs live conversion. Choose FLAC or Opus on the normal Collector connection, or prepare a saved copy.');
    if(requested!=='saved'&&(requested==='flac'?track.mediaType!=='audio/flac':track.opusBitrate!==preference.bitrate))
     throw new Error('Live conversion is unavailable on this separate audio connection. Choose Saved audio to play the existing file.');
    await checkTrack(track,controller.signal,renew);
    if(!element.canPlayType(track.mediaType))throw new Error('This browser cannot play this format. Choose another playback format or download the track.');
    url=await playbackUrl(musicTrackUrl(track),controller.signal);
   }
   if(token!==generation.current||audio.current!==element)return;
   streamCheck.current=checkPath?{track:track.id,path:checkPath}:undefined;
   setRouteLabel(requested==='saved'?route:`${requested==='flac'?'FLAC':`Opus · ${preference.bitrate} kbps`} · ${route}`);
   if(loaded.current!==url){
    if(liveOffset!==undefined){live.current={offset:liveOffset,duration:track.seconds||0};setPosition(liveOffset);setDuration(track.seconds||0);}
    else pendingSeek.current=previousPosition>0?{element,position:previousPosition}:undefined;
    element.crossOrigin='anonymous';element.src=url;element.load();loaded.current=url;
   }
   await element.play();
  }catch(e){
   if(controller.signal.aborted||token!==generation.current||audio.current!==element)return;
   element.pause();if(element.error)loaded.current='';setState('Paused');setError(requestErrorMessage(e));if(isApiRequestError(e)&&e.status===401)expired.current();else if(element.error&&requested==='saved')void diagnosePlaybackFailure(track,element,token);
  }
 }
 useEffect(()=>{if(queue.playRequest&&queue.playRequest===current?.id)void play(current);},[queue.playRequest]);
 function pause(){clearTimeout(liveSeek.current);liveSeek.current=undefined;generation.current++;request.current?.abort();const element=audio.current;element?.pause();if(element&&live.current){resume.current=live.current.offset+element.currentTime;element.removeAttribute('src');element.load();loaded.current='';}setState('Paused');}
 function seekTo(value:number){
  const element=audio.current;if(!element||!current)return;
  if(live.current){
   setPosition(value);
   if(element.paused){resume.current=value;return;}
   clearTimeout(liveSeek.current);liveSeek.current=undefined;liveSeek.current=setTimeout(()=>void play(current,false,value),180);
  }else try{element.currentTime=value;setPosition(element.currentTime);}catch{setError('This browser could not seek in the file.');}
 }
 function toggle(){const element=audio.current;if(!element||!current)return;if(!element.paused||state==='Loading')pause();else void play(current);}
 function move(delta:number,automatic=false){const track=queue.tracks[index+delta];if(track)void play(track);else if(automatic)setState('Queue finished');}
 useMusicMediaSession({track:current,audio,enabled:visible&&!!current&&active.current===current.id&&state!=='Ready'&&state!=='Queue finished',play:()=>{if(current)void play(current);},pause,stop:release,previous:index>0?()=>move(-1):undefined,next:index>=0&&index<queue.tracks.length-1?()=>move(1):undefined});
 if(!visible)return null;
 return <section ref={dock} className="music-player music-dock" aria-label="Music player">
  <audio ref={bindAudio} crossOrigin="anonymous" preload="none" data-representation-id={current?.representationId} onLoadedMetadata={e=>{setDuration(live.current?.duration||(Number.isFinite(e.currentTarget.duration)?(live.current?.offset||0)+e.currentTarget.duration:0));const pending=pendingSeek.current;if(pending?.element===e.currentTarget){pendingSeek.current=undefined;try{e.currentTarget.currentTime=pending.position;}catch{setError('This browser could not restore the playback position.');}}}} onDurationChange={e=>setDuration(live.current?.duration||(Number.isFinite(e.currentTarget.duration)?(live.current?.offset||0)+e.currentTarget.duration:0))} onTimeUpdate={e=>{if(loaded.current&&!liveSeek.current)setPosition((live.current?.offset||0)+e.currentTarget.currentTime);}} onPlay={e=>{if(!e.currentTarget.paused)setState('Now playing');}} onPause={e=>{if(e.currentTarget.paused&&!e.currentTarget.ended&&e.currentTarget.currentSrc)setState('Paused');}} onEnded={e=>{if(e.currentTarget.ended)move(1,true);}} onError={e=>{if(e.currentTarget.error&&e.currentTarget.currentSrc&&current){if(!live.current&&remotePlaybackActive(musicTrackUrl(current))&&!recoveredGrant.current){recoveredGrant.current=true;void play(current,true);return;}loaded.current='';setState('Paused');setError('Playback failed. Use Play to retry, or download the track from its album.');if(!e.currentTarget.currentSrc.includes('/music-stream/'))void diagnosePlaybackFailure(current,e.currentTarget,generation.current);}}}/>
  {expanded&&<section className="music-queue" aria-label="Currently playing list" onKeyDown={e=>{if(e.key==='Escape'){e.stopPropagation();setExpanded(false);queueButton.current?.focus();}}}><header><div><h3>Currently playing</h3><small>Queue saved in this browser.</small></div><button onClick={()=>{clearMusicQueue();setExpanded(false);queueButton.current?.focus();}}>Clear queue</button></header>{!queue.tracks.length?<p>Add tracks from an album’s info view.</p>:<ol>{queue.tracks.map((track,i)=><li key={track.id} aria-current={track.id===queue.current?'true':undefined}><button className="queue-track" onClick={()=>void play(track)}><strong>{track.position}. {track.title}</strong><small>{track.album}</small></button><div className="queue-actions"><button aria-label={`Move ${track.title} up`} disabled={i===0} onClick={()=>reorderMusicTrack(track.id,-1)}>↑</button><button aria-label={`Move ${track.title} down`} disabled={i===queue.tracks.length-1} onClick={()=>reorderMusicTrack(track.id,1)}>↓</button><button aria-label={`Remove ${track.title} from queue`} onClick={()=>{removeMusicTrack(track.id);requestAnimationFrame(()=>{const buttons=dock.current?.querySelectorAll<HTMLButtonElement>('.queue-track');(buttons?.[Math.min(i,(buttons?.length||1)-1)]||queueButton.current)?.focus();});}}>Remove</button></div></li>)}</ol>}</section>}
  <div className="dock-main"><div className="dock-track">{current?.artwork&&<img src={`/api/v1/collections/${current.collectionId}/assets/${current.artwork}/thumbnail`} alt=""/>}<p className="now-playing" role="status"><span>{current?state:'Your music queue'}</span><strong>{current?`${current.position}. ${current.title}`:'Add tracks from an album'}</strong>{current&&<small>{current.album}</small>}</p></div>
   <div><div className="transport"><button aria-label="Previous track" disabled={index<=0} onClick={()=>move(-1)}>Previous</button><button className="primary" aria-label={state==='Now playing'||state==='Loading'?'Pause':'Play'} disabled={!current} onClick={toggle}>{state==='Now playing'||state==='Loading'?'Pause':'Play'}</button><button aria-label="Next track" disabled={index<0||index>=queue.tracks.length-1} onClick={()=>move(1)}>Next</button></div><label className="seek"><span>{time(position)}</span><input aria-label={`Seek in ${current?.title||'track'}`} type="range" min="0" max={duration||0} step="0.05" value={Math.min(position,duration||0)} disabled={(!loaded.current&&!live.current)||duration<=0} onChange={e=>seekTo(Number(e.target.value))}/><span>{time(duration)}</span></label></div>
   <div className="dock-options"><button ref={queueButton} className="queue-toggle" aria-expanded={expanded} onClick={()=>setExpanded(v=>!v)}>Queue ({queue.tracks.length})</button><PlaybackFormatControl compact/></div>
  </div>{routeLabel&&<small>{routeLabel}</small>}{error&&<p role="alert" className="error dock-error">{error}</p>}
 </section>;
}
