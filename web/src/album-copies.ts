// Rules for which copy and which format a music album plays from, and the
// track overrides a choice saves. The music tracks panel reads the current
// tracks, calls these and sends the result; nothing here touches the page.
import type {components} from './generated/api';

type Availability=components['schemas']['MusicTrackAvailability'];
export type TrackRow=Pick<Availability,'capture_id'|'source_representation_id'|'track_position'|'disc_position'|'display_position'|'profile'|'album_selection'|'audio_format'|'source_label'|'state'|'can_stream_original'>;
type Source={positions:Set<number>;discs:Set<number>;used:boolean};
export type AlbumCopy={key:string;sources:Map<string,Source>;used:boolean;label:string};
export type FormatChoice={id:string;label:string;codecs:string[];current:string};
// One track's album selection: true uses it, false leaves it out, null lets the
// server decide again.
export type SelectionChange={source:string;position:number;selected:boolean|null};
export type TrackOverride={source_representation_id:string;track_position:number;title:unknown;disc_position:unknown;display_position:unknown;album_selected:boolean|null};

// Copies the album could play from, when it holds more than one, such as a
// CD set and a digital purchase of the same album. Originals are alternative
// copies only when they cover the same album disc: CD rips form one copy, and
// each digital purchase joins the first copy whose discs it doesn't repeat, so
// two purchases holding disc 1 and disc 2 stay one copy. Saved formats are not
// compared, only originals.
export function albumCopies(rows:readonly TrackRow[],sourceLabel:(id:string)=>string):AlbumCopy[]{
 const fileSources=new Set(rows.filter(row=>!row.profile).map(row=>row.source_representation_id));
 const sources=new Map<string,Source>();
 for(const row of rows){
  const file=fileSources.has(row.source_representation_id);
  if(file?!!row.profile:row.profile?.codec!=='flac')continue;
  const source=sources.get(row.source_representation_id)||{positions:new Set<number>(),discs:new Set<number>(),used:false};
  source.positions.add(row.track_position);source.discs.add(row.disc_position??1);if(row.album_selection!=='excluded')source.used=true;
  sources.set(row.source_representation_id,source);}
 type Copy={key:string;sources:Map<string,Source>;discs:Set<number>;rips:number;files:string[]};
 const copies:Copy[]=[];
 const rips=[...sources].filter(([id])=>!fileSources.has(id));
 if(rips.length)copies.push({key:'discs',sources:new Map(rips),discs:new Set(rips.flatMap(([,source])=>[...source.discs])),rips:rips.length,files:[]});
 for(const [id,source] of sources){if(!fileSources.has(id))continue;
  let copy=copies.find(copy=>![...source.discs].some(disc=>copy.discs.has(disc)));
  if(!copy){copy={key:id,sources:new Map(),discs:new Set(),rips:0,files:[]};copies.push(copy);}
  copy.sources.set(id,source);source.discs.forEach(disc=>copy!.discs.add(disc));copy.files.push(id);}
 if(copies.length<2)return [];
 return copies.map(copy=>({key:copy.key,sources:copy.sources,used:[...copy.sources.values()].some(source=>source.used),
  label:[copy.rips?`CD rips (${copy.rips} ${copy.rips===1?'disc':'discs'})`:'',copy.files.length===1?`Digital files · ${sourceLabel(copy.files[0])}`:copy.files.length?`${copy.files.length} digital purchases`:''].filter(Boolean).join(' + ')}));
}

// The copy the album plays from, or '' when none or several are in use.
export function currentCopy(copies:readonly AlbumCopy[]):string{
 const used=copies.filter(choice=>choice.used);
 return used.length===1?used[0].key:'';
}

// Leaves every other copy out of the album, so playback and lists use only the
// chosen one; the others stay saved and playable on their own. An empty key
// puts back copies that were left out whole.
export function copySelectionChanges(copies:readonly AlbumCopy[],key:string):SelectionChange[]{
 const changes:SelectionChange[]=[];
 for(const choice of copies)for(const [id,source] of choice.sources){
  const selected=key&&choice.key!==key?false:!source.used?null:undefined;
  if(selected!==undefined)for(const position of source.positions)changes.push({source:id,position,selected});
 }
 return changes;
}

// Originals holding the same tracks in more than one format, for example a
// purchase with MP3 and WAV folders. Each offers one album-wide format choice.
export function formatChoices(rows:readonly TrackRow[]):FormatChoice[]{
 const sources=new Map<string,{label:string;codecs:Map<string,{included:number;total:number}>}>();
 for(const row of rows){if(row.profile||!row.audio_format)continue;
  const source=sources.get(row.source_representation_id)||{label:row.source_label,codecs:new Map()};
  const count=source.codecs.get(row.audio_format.codec)||{included:0,total:0};
  count.total++;if(row.album_selection==='included')count.included++;
  source.codecs.set(row.audio_format.codec,count);sources.set(row.source_representation_id,source);}
 // A copy the album doesn't use offers no format choice; picking one there
 // would quietly bring that copy back. Plays from handles copies.
 return [...sources.entries()].filter(([,source])=>source.codecs.size>1&&[...source.codecs.values()].some(count=>count.included>0)).map(([id,source])=>{
  const codecs=[...source.codecs.entries()];
  const used=codecs.filter(([,count])=>count.included>0);
  return {id,label:source.label,codecs:codecs.map(([codec])=>codec),current:used.length===1&&used[0][1].included===used[0][1].total?used[0][0]:''};
 });
}

// Uses one format for every track of one original at once, such as WAV instead
// of MP3. An empty codec lets the server pick the best quality again.
export function formatSelectionChanges(rows:readonly TrackRow[],source:string,codec:string):SelectionChange[]{
 return rows.filter(row=>row.source_representation_id===source&&!row.profile)
  .map(row=>({source,position:row.track_position,selected:codec?row.audio_format?.codec===codec:null}));
}

// Applies selection changes to the saved track overrides. Each changed track
// keeps its title, disc and number edits; overrides for other tracks are left
// alone; an override with nothing left in it is dropped rather than saved empty.
export function mergeTrackOverrides(existing:readonly Record<string,unknown>[],changes:readonly SelectionChange[]):Record<string,unknown>[]{
 const touched=new Set(changes.map(change=>`${change.source}\n${change.position}`));
 const kept=existing.filter(value=>!touched.has(`${String(value.source_representation_id)}\n${Number(value.track_position)}`));
 const updated=changes.map(change=>{
  const prior=existing.find(value=>value.source_representation_id===change.source&&value.track_position===change.position);
  return {source_representation_id:change.source,track_position:change.position,title:prior?.title??null,disc_position:prior?.disc_position??null,display_position:prior?.display_position??null,album_selected:change.selected} satisfies TrackOverride;
 }).filter(value=>value.title!=null||value.disc_position!=null||value.display_position!=null||value.album_selected!=null);
 return [...kept,...updated];
}

// Which ready file plays a track: an original first, then FLAC, then any other
// format the player can decode. canPlay stands in for the browser's own check.
export function preferredAvailable<T extends TrackRow>(group:readonly T[],canPlay:(mediaType:string)=>boolean):T|undefined{
 const available=group.filter(row=>row.state.status==='available'&&canPlay(row.state.media_type));
 return available.find(row=>!row.profile)||available.find(row=>row.profile?.codec==='flac')||available[0];
}

export function discKey(row:Pick<TrackRow,'capture_id'|'source_representation_id'>):string{return `${row.capture_id}/${row.source_representation_id}`;}

// Groups tracks by disc, with every format of a track together, in album order:
// discs by their place in the album, tracks by disc then number.
export function groupDiscs<T extends TrackRow>(rows:readonly T[],discOrder:(key:string)=>number):(readonly [string,T[][]])[]{
 const grouped=new Map<string,Map<number,T[]>>();
 for(const row of rows){const key=discKey(row);const tracks=grouped.get(key)||new Map<number,T[]>();const variants=tracks.get(row.track_position)||[];variants.push(row);tracks.set(row.track_position,variants);grouped.set(key,tracks);}
 return [...grouped.entries()].map(([key,tracks])=>[key,[...tracks.entries()].sort(([,a],[,b])=>(a[0].disc_position??1)-(b[0].disc_position??1)||(a[0].display_position??a[0].track_position)-(b[0].display_position??b[0].track_position)).map(([,variants])=>variants)] as const).sort(([a],[b])=>discOrder(a)-discOrder(b)||a.localeCompare(b));
}
