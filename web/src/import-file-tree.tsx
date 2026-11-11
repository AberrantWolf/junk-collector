import React,{useState} from '@collector/runtime';
import type {components} from './generated/api';
import {bytes} from './work-progress';

type S=components['schemas'];
type Track=S['MusicImportTrack'];
type File=S['UploadFile'];
type Node={name:string;path:string;fileIndex?:number;children:Map<string,Node>;audio:number[]};
const audio=(name:string)=>/\.(flac|wav|mp3)$/i.test(name);
const pageSize=50;
const pathDisc=(name:string):number|null=>{
 for(const part of name.split('/').slice(0,-1).reverse()){
  const match=/^(?:disc|cd)[ _-]*(\d+)$/i.exec(part);
  if(match&&Number(match[1])>0&&Number(match[1])<=65535)return Number(match[1]);
 }
 return null;
};


function tree(files:File[]):Node{
 const root:Node={name:'All files',path:'',children:new Map(),audio:[]};
 files.forEach((file,index)=>{
  const components=file.name.split('/');
  const parts=components.length>32?[...components.slice(0,31),components.slice(31).join('/')]:components;let node=root;
  if(audio(file.name))node.audio.push(index);
  parts.forEach((part,position)=>{
   let child=node.children.get(part);
   if(!child){child={name:part,path:parts.slice(0,position+1).join('/'),children:new Map(),audio:[]};node.children.set(part,child);}
   node=child;if(audio(file.name))node.audio.push(index);
   if(position===parts.length-1)node.fileIndex=index;
  });
 });
 return root;
}

function SelectionBox({checked,partial,label,disabled,onChange}:{checked:boolean;partial:boolean;label:string;disabled:boolean;onChange:()=>void}){
 return <input ref={element=>{if(element)element.indeterminate=partial;}} type="checkbox" checked={checked} aria-label={label} disabled={disabled} onChange={onChange}/>;
}

function DiscChoice({label,value,discs,disabled,onChange}:{label:string;value:number|undefined;discs:number[];disabled:boolean;onChange:(disc:number)=>void}){
 const next=Math.max(0,...discs)+1;
 return <label className="import-disc-choice">Disc<select aria-label={label} disabled={disabled} value={value??''} onChange={event=>onChange(Number(event.target.value))}>
  <option value="" disabled>Automatic / mixed</option>{discs.map(disc=><option key={disc} value={disc}>Disc {disc}</option>)}{next<=65535&&<option value={next}>New disc ({next})</option>}
 </select></label>;
}

type TreeProps={files:File[];tracks:Track[];disabled:boolean;onTracks:(tracks:Track[])=>void;onVisibility:(index:number,privateFile:boolean)=>void;onMove:(index:number)=>void};
function Branch({node,files,tracks,selected,discs,disabled,onTracks,onVisibility,onMove}:{node:Node;selected:Map<string,Track>;discs:number[]}&TreeProps){
 const [open,setOpen]=useState(false);const [page,setPage]=useState(0);
 const included=node.audio.filter(index=>selected.has(files[index].name));
 const discValues=new Set(included.map(index=>selected.get(files[index].name)?.disc_position));
 const disc=discValues.size===1?[...discValues][0]??undefined:undefined;
 function choose(include:boolean){
  const affected=new Set(node.audio.map(index=>files[index].name));
  const retained=tracks.filter(track=>!affected.has(track.file_name));
  if(include)node.audio.forEach(index=>retained.push(selected.get(files[index].name)||{file_name:files[index].name,disc_position:pathDisc(files[index].name)}));
  onTracks(retained);
 }
 function assign(disc:number){
  const affected=new Set(node.audio.map(index=>files[index].name));
  onTracks(tracks.map(track=>affected.has(track.file_name)?{...track,disc_position:disc}:track));
 }
 if(node.fileIndex!==undefined){
  const index=node.fileIndex;const file=files[index];
  return <li className="import-tree-file"><div className="import-tree-line">
   {audio(file.name)?<label><SelectionBox checked={selected.has(file.name)} partial={false} label={`Use for album playback: ${file.name}`} disabled={disabled} onChange={()=>choose(!selected.has(file.name))}/>{node.name}</label>:<span>{node.name}</span>}
   <small>{bytes(file.bytes)}</small>
   {audio(file.name)&&selected.has(file.name)&&<DiscChoice label={`Disc for ${file.name}`} value={disc} discs={discs} disabled={disabled} onChange={assign}/>}
   {audio(file.name)&&index>0&&<button disabled={disabled} onClick={()=>onMove(index)} aria-label={`Move ${file.name} earlier`}>Move earlier</button>}
   <label><input type="checkbox" checked={file.visibility==='curators'} aria-label={`Curators only: ${file.name}`} disabled={disabled} onChange={event=>onVisibility(index,event.target.checked)}/>Curators only</label>
  </div></li>;
 }
 const children=open?[...node.children.values()]:[];
 return <li><div className="import-tree-line">
  {!!node.audio.length&&<SelectionBox checked={included.length===node.audio.length} partial={included.length>0&&included.length<node.audio.length} label={`Use folder for album playback: ${node.path||'All files'}`} disabled={disabled} onChange={()=>choose(included.length!==node.audio.length)}/>}
  <button type="button" className="import-folder-toggle" aria-expanded={open} onClick={()=>setOpen(value=>!value)}>{open?'▾':'▸'} {node.name}</button>
  {!!node.audio.length&&<small>{included.length} / {node.audio.length} audio files selected</small>}
  {!!included.length&&<DiscChoice label={`Disc for folder ${node.path||'All files'}`} value={disc} discs={discs} disabled={disabled} onChange={assign}/>}
 </div>{open&&<ul>{children.slice(page*pageSize,(page+1)*pageSize).map(child=><Branch key={child.path} node={child} files={files} tracks={tracks} selected={selected} discs={discs} disabled={disabled} onTracks={onTracks} onVisibility={onVisibility} onMove={onMove}/>)}{children.length>pageSize&&<li><button disabled={!page} onClick={()=>setPage(value=>value-1)}>Previous entries</button> <span>Page {page+1} of {Math.ceil(children.length/pageSize)}</span> <button disabled={(page+1)*pageSize>=children.length} onClick={()=>setPage(value=>value+1)}>Next entries</button></li>}</ul>}</li>;
}

export default function ImportFileTree({files,tracks,layout,reviewed,disabled,onTracks,onVisibility,onMove}:{layout:S['MusicImportLayout'];reviewed:boolean}&TreeProps){
 const [open,setOpen]=useState(false);
 const root=React.useMemo(()=>open?tree(files):undefined,[files,open]);
 const selected=React.useMemo(()=>new Map(tracks.map(track=>[track.file_name,track])),[tracks]);
 const discs=React.useMemo(()=>[...new Set([1,...tracks.flatMap(track=>track.disc_position?[track.disc_position]:[])])].sort((a,b)=>a-b),[tracks]);
 const formats=React.useMemo(()=>['flac','wav','mp3'].filter(format=>files.some(file=>file.name.toLowerCase().endsWith('.'+format))),[files]);
 function selectFormat(format:string){
  const guesses=new Map(layout.tracks.map(track=>[track.file_name,track.disc_position]));
  onTracks(files.filter(file=>file.name.toLowerCase().endsWith('.'+format)).map(file=>({file_name:file.name,disc_position:selected.get(file.name)?.disc_position??guesses.get(file.name)??pathDisc(file.name)})));
 }
 return <details onToggle={event=>setOpen(event.currentTarget.open)}><summary>Files and album playback · {tracks.length} selected{layout.needs_attention&&!reviewed?' · Needs attention':''}</summary>
  {open&&<><p>All {files.length} originals will be preserved. Checked audio files form album playback. Choose disc numbers for multi-disc albums.</p>
   {layout.notices.map(notice=><p key={notice} className="notice">{notice}</p>)}
   <div className="care-actions">{formats.map(format=><button key={format} disabled={disabled} onClick={()=>selectFormat(format)}>Select {format.toUpperCase()}</button>)}<button disabled={disabled||!tracks.length} onClick={()=>onTracks(tracks)}>Confirm playback selection</button></div>
   {!tracks.length&&<p role="alert">Select at least one audio file for album playback.</p>}
   {root&&<ul className="import-file-tree" aria-label="Album files"><Branch node={root} files={files} tracks={tracks} selected={selected} discs={discs} disabled={disabled} onTracks={onTracks} onVisibility={onVisibility} onMove={onMove}/></ul>}
  </>}
 </details>;
}
