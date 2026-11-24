import React,{useEffect,useState} from './runtime';

// The layout renders one <MediaDock/>. Players (music today; video or an
// emulator later) register a surface here and decide for themselves when they
// have something to show. Surfaces stay mounted so playback continues while
// their bar is hidden. Only one surface plays at a time: starting one asks the
// others to pause through claimMediaFocus.
export type MediaSurfaceProps={onSessionExpired:()=>void};
type Surface=React.ComponentType<MediaSurfaceProps>;
const surfaces=new Map<string,Surface>();
const listeners=new Set<()=>void>();
const focusListeners=new Set<(owner:string)=>void>();

export function registerMediaSurface(id:string,surface:Surface){
 surfaces.set(id,surface);
 for(const listener of listeners)listener();
}

export function claimMediaFocus(owner:string){
 for(const listener of focusListeners)listener(owner);
}

export function useMediaFocus(owner:string,pause:()=>void){
 const latest=React.useRef(pause);latest.current=pause;
 useEffect(()=>{
  const listener=(claimant:string)=>{if(claimant!==owner)latest.current();};
  focusListeners.add(listener);
  return()=>{focusListeners.delete(listener);};
 },[owner]);
}

export default function MediaDock(props:MediaSurfaceProps){
 const [,setVersion]=useState(0);
 useEffect(()=>{
  const listener=()=>setVersion(value=>value+1);
  listeners.add(listener);
  return()=>{listeners.delete(listener);};
 },[]);
 return <>{[...surfaces.entries()].map(([id,Surface])=><Surface key={id} {...props}/>)}</>;
}
