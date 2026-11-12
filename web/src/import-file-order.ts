import type {components} from './generated/api';

type Track=components['schemas']['MusicImportTrack'];
const audioFormats=['flac','wav','mp3'];
const names=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
export function audioFormat(name:string){
 const suffix=name.slice(name.lastIndexOf('.')+1).toLowerCase();
 return name.includes('.')&&audioFormats.includes(suffix)?suffix:undefined;
}
export function compareImportFiles(a:string,b:string){
 const first=audioFormats.indexOf(audioFormat(a)??'');
 const second=audioFormats.indexOf(audioFormat(b)??'');
 return (first<0?audioFormats.length:first)-(second<0?audioFormats.length:second)||names.compare(a,b);
}

export const orderGroup=(path:string,format:string|null)=>JSON.stringify([path,format]);
export const trackDisc=(track:Track)=>track.disc_position??1;

export function automaticTrackNumbers(files:readonly {name:string}[],tracks:readonly Track[]){
 const selected=new Map(tracks.map(track=>[track.file_name,track]));
 const reserved=new Map<number,Set<number>>();
 for(const track of tracks){
  if(track.display_position==null)continue;
  const disc=trackDisc(track);
  const positions=reserved.get(disc)??new Set<number>();
  positions.add(track.display_position);
  reserved.set(disc,positions);
 }
 const numbers=new Map<string,number>();
 const nextOnDisc=new Map<number,number>();
 for(const file of files){
  const track=selected.get(file.name);
  if(!track||track.display_position!=null)continue;
  const disc=trackDisc(track);
  let number=nextOnDisc.get(disc)??1;
  while(reserved.get(disc)?.has(number))number++;
  numbers.set(file.name,number);
  nextOnDisc.set(disc,number+1);
 }
 return numbers;
}

export function freezeTrackOrder(files:readonly {name:string}[],tracks:Track[],manualGroups:readonly string[]){
 const folderParents=new Set<string>();
 for(const group of manualGroups){
  const [path,format]=JSON.parse(group) as [string,string|null];
  if(format===null)folderParents.add(path);
 }
 const orderedDiscs=new Set<number>();
 for(const track of tracks){
  if(track.display_position!=null)orderedDiscs.add(trackDisc(track));
  const parts=track.file_name.split('/');
  for(let i=0;i<=parts.length-2;i++){
   if(folderParents.has(parts.slice(0,i).join('/')))orderedDiscs.add(trackDisc(track));
  }
 }
 if(!orderedDiscs.size)return tracks;
 const numbers=automaticTrackNumbers(files,tracks);
 return tracks.map(track=>{
  const disc=trackDisc(track);
  if(!orderedDiscs.has(disc))return track;
  return {...track,disc_position:disc,display_position:track.display_position??numbers.get(track.file_name)};
 });
}
