import {useEffect,useRef,type RefObject} from 'react';
import type {QueuedTrack} from './music-queue';

const actions:MediaSessionAction[]=['play','pause','stop','previoustrack','nexttrack','seekto','seekbackward','seekforward'];

// This browser adapter owns only the music player's OS-facing presentation.
// Playback and authorization still run through the player's existing actions.
export function clearMusicMediaSession(){
 if(!('mediaSession' in navigator))return;
 const session=navigator.mediaSession;
 session.metadata=null;
 session.playbackState='none';
 try{session.setPositionState?.();}catch{/* Optional on older browsers. */}
 for(const action of actions)try{session.setActionHandler(action,null);}catch{/* Unsupported action. */}
}

type Options={
 track:QueuedTrack|undefined;
 audio:RefObject<HTMLAudioElement|null>;
 enabled:boolean;
 play:()=>void;
 pause:()=>void;
 stop:()=>void;
 previous:(()=>void)|undefined;
 next:(()=>void)|undefined;
};

export function useMusicMediaSession(options:Options){
 const latest=useRef(options);latest.current=options;
 const {track,audio,enabled}=options;
 const previous=!!options.previous,next=!!options.next;
 useEffect(()=>{
  if(!('mediaSession' in navigator))return;
  const element=audio.current;
  if(!enabled||!track||!element){clearMusicMediaSession();return;}
  const session=navigator.mediaSession;
  if(typeof MediaMetadata!=='undefined')session.metadata=new MediaMetadata({
   title:track.title,artist:track.artist||'',album:track.album,
   artwork:track.artwork?[{src:new URL(`/api/v1/collections/${track.collectionId}/assets/${track.artwork}/thumbnail`,location.href).href}]:[],
  });
  function position(){
   const duration=element!.duration;
   try{
    if(Number.isFinite(duration)&&duration>0&&element!.playbackRate>0)
     session.setPositionState?.({duration,playbackRate:element!.playbackRate,position:Math.max(0,Math.min(element!.currentTime,duration))});
    else session.setPositionState?.();
   }catch{/* Position reporting is optional; the audio element remains authoritative. */}
  }
  function synchronize(){if(latest.current.audio.current!==element||latest.current.track?.id!==track!.id||!latest.current.enabled)return;session.playbackState=element!.paused?'paused':'playing';position();}
  function seek(value:number){if(Number.isFinite(value)&&Number.isFinite(element!.duration)&&element!.duration>0){element!.currentTime=Math.max(0,Math.min(value,element!.duration));position();}}
  const handlers:Partial<Record<MediaSessionAction,MediaSessionActionHandler>>={
   play:()=>latest.current.play(),pause:()=>latest.current.pause(),stop:()=>latest.current.stop(),
   previoustrack:previous?()=>latest.current.previous?.():undefined,
   nexttrack:next?()=>latest.current.next?.():undefined,
   seekto:details=>{if(details.seekTime!==undefined)seek(details.seekTime);},
   seekbackward:details=>seek(element!.currentTime-(details.seekOffset??10)),
   seekforward:details=>seek(element!.currentTime+(details.seekOffset??10)),
  };
  for(const action of actions)try{session.setActionHandler(action,handlers[action]||null);}catch{/* Unsupported action. */}
  const events=['play','pause','loadedmetadata','durationchange','timeupdate','seeked','ratechange'];
  for(const event of events)element.addEventListener(event,synchronize);
  synchronize();
  return()=>{for(const event of events)element.removeEventListener(event,synchronize);clearMusicMediaSession();};
 },[enabled,track?.id,track?.title,track?.artist,track?.album,track?.artwork,track?.collectionId,previous,next]);
}
