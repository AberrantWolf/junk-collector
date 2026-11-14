import {useSyncExternalStore} from 'react';

export type PlaybackFormat='original'|'high_bandwidth'|'low_bandwidth';
type Preference={format:PlaybackFormat;bitrate:number};
const defaults:Preference={format:'high_bandwidth',bitrate:128};
const listeners=new Set<()=>void>();
let owner:string|undefined;
let preference=defaults;
const key=()=>`collector.playback-format.v2.${owner}`;
export const validOpusBitrate=(rate:number)=>Number.isInteger(rate)&&rate>=6&&rate<=256;
const validFormat=(format:unknown):format is PlaybackFormat=>format==='original'||format==='high_bandwidth'||format==='low_bandwidth';

export function setPlaybackPreferenceUser(user?:string){
 if(owner===user)return;
 owner=user;preference=defaults;
 if(user)try{
  const raw=localStorage.getItem(key());
  const saved=raw&&raw.length<128?JSON.parse(raw):null;
  if(saved&&validFormat(saved.format)&&validOpusBitrate(saved.bitrate))preference={format:saved.format,bitrate:saved.bitrate};
 }catch{/* Playback preferences can work for this page without storage. */}
 for(const listener of listeners)listener();
}
export function setPlaybackPreference(next:Preference){
 if(!owner||!validFormat(next.format)||!validOpusBitrate(next.bitrate))return;
 preference=next;
 try{localStorage.setItem(key(),JSON.stringify(next));}catch{/* Still usable in this page. */}
 for(const listener of listeners)listener();
}
export function getPlaybackPreference(){return preference;}
export function usePlaybackPreference(){
 return useSyncExternalStore(listener=>{listeners.add(listener);return()=>{listeners.delete(listener);};},getPlaybackPreference);
}
