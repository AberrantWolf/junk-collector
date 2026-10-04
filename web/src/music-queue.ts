import {useSyncExternalStore} from 'react';

// Personal browser activity: immutable coordinates, never paths or media bytes.
export type QueuedTrack={id:string;itemId:string;collectionId:string;album:string;title:string;position:number;representationId:string;fileIndex:number;mediaType:string;artwork?:string;original?:boolean};
export const MAX_MUSIC_QUEUE=5000;
type Queue={tracks:QueuedTrack[];current:string|null;playRequest?:string};
const empty:Queue={tracks:[],current:null};
let owner:string|undefined;
let state:Queue=empty;
const listeners=new Set<()=>void>();
const storageKey=()=>`collector.music-queue.v1.${owner}`;
const uuid=(v:unknown):v is string=>typeof v==='string'&&/^[0-9a-f-]{36}$/i.test(v);
function valid(v:QueuedTrack){return v&&uuid(v.id)&&uuid(v.itemId)&&uuid(v.collectionId)&&uuid(v.representationId)&&Number.isInteger(v.fileIndex)&&v.fileIndex>=0&&Number.isInteger(v.position)&&v.position>=0&&typeof v.album==='string'&&v.album.length<=4096&&typeof v.title==='string'&&v.title.length<=4096&&typeof v.mediaType==='string'&&v.mediaType.length<100&&(!v.artwork||uuid(v.artwork))&&(v.original===undefined||typeof v.original==='boolean');}
function publish(next:Queue,persist=true){state=next;if(owner&&persist)try{localStorage.setItem(storageKey(),JSON.stringify(next));}catch{/* Disabled/full local storage must not prevent playback. */}for(const listener of listeners)listener();}
export function setMusicQueueUser(user?:string){
 if(owner===user)return;owner=user;let next=empty;
 if(user)try{const text=localStorage.getItem(storageKey());if(text&&text.length<8_000_000){const value=JSON.parse(text);if(Array.isArray(value.tracks)&&value.tracks.length<=MAX_MUSIC_QUEUE&&value.tracks.every(valid)&&new Set(value.tracks.map((v:QueuedTrack)=>v.id)).size===value.tracks.length)next={tracks:value.tracks,current:value.tracks.some((v:QueuedTrack)=>v.id===value.current)?value.current:value.tracks[0]?.id||null};}}catch{/* Malformed local state starts empty. */}
 publish(next,false);
}
export function useMusicQueue(){return useSyncExternalStore(listener=>{listeners.add(listener);return()=>{listeners.delete(listener);};},()=>state);}
export function addMusicTracks(tracks:Omit<QueuedTrack,'id'>[]){if(!owner)return 0;const additions=tracks.slice(0,MAX_MUSIC_QUEUE-state.tracks.length).map(track=>({...track,id:crypto.randomUUID()}));publish({tracks:[...state.tracks,...additions],current:state.current||additions[0]?.id||null});return additions.length;}
export function selectMusicTrack(id:string){if(state.tracks.some(t=>t.id===id))publish({...state,current:id});}
export function removeMusicTrack(id:string){const index=state.tracks.findIndex(t=>t.id===id);const tracks=state.tracks.filter(t=>t.id!==id);publish({tracks,current:state.current===id?tracks[index]?.id||tracks[index-1]?.id||null:state.current});}
export function reorderMusicTrack(id:string,delta:number){const tracks=[...state.tracks];const index=tracks.findIndex(t=>t.id===id);const next=index+delta;if(index<0||next<0||next>=tracks.length)return;[tracks[index],tracks[next]]=[tracks[next],tracks[index]];publish({...state,tracks});}
export function clearMusicQueue(){publish(empty);}
export function musicTrackUrl(track:QueuedTrack){return `/api/v1/items/${track.itemId}/representations/${track.representationId}/${track.fileIndex}`;}

export function audioMediaType(name:string):string|undefined{return ({flac:'audio/flac',mp3:'audio/mpeg',wav:'audio/wav',opus:'audio/ogg; codecs=opus',ogg:'audio/ogg'} as Record<string,string>)[name.split('.').pop()?.toLowerCase()||''];}

// Explicit user playback replaces the queue atomically; reload never restores autoplay.
export function startMusicTracks(tracks:Omit<QueuedTrack,'id'>[],shuffle=false){
 if(!owner||!tracks.length||tracks.length>MAX_MUSIC_QUEUE)return false;
 const next=tracks.map(track=>({...track,id:crypto.randomUUID()}));
 if(shuffle)for(let i=next.length-1;i>0;i--){const j=Math.floor(Math.random()*(i+1));[next[i],next[j]]=[next[j],next[i]];}
 publish({tracks:next,current:next[0].id,playRequest:next[0].id});return true;
}
