import React,{useId,useRef,useState} from '@collector/runtime';
import type {components} from './generated/api';
import {bytes} from './work-progress';
import {audioFormat,automaticTrackNumbers,compareImportFiles,orderGroup,trackDisc} from './import-file-order';

type S=components['schemas'];
type Track=S['MusicImportTrack'];
const maxMusicNumber=65535;
type File=S['UploadFile'];
type Node={name:string;path:string;fileIndex?:number;children:Map<string,Node>;audio:number[]};
const pageSize=50;
const pathDisc=(name:string):number|null=>{
 for(const part of name.split('/').slice(0,-1).reverse()){
  const match=/^(?:disc|cd)[ _-]*(\d+)$/i.exec(part);
  if(match&&Number(match[1])>0&&Number(match[1])<=maxMusicNumber)return Number(match[1]);
 }
 return null;
};


const audioFolder=(node:Node)=>node.fileIndex===undefined&&node.audio.length>0;

function orderedAudio(node:Node):number[]{
 if(node.fileIndex!==undefined)return node.audio;
 return [...node.children.values()].flatMap(orderedAudio);
}

function tree(files:File[],manualGroups:readonly string[]){
 const manualOrder=new Set(manualGroups);
 const root:Node={name:'All files',path:'',children:new Map(),audio:[]};
 files.forEach((file,index)=>{
  const components=file.name.split('/');
  const parts=components.length>32?[...components.slice(0,31),components.slice(31).join('/')]:components;let node=root;
  if(audioFormat(file.name))node.audio.push(index);
  parts.forEach((part,position)=>{
   let child=node.children.get(part);
   if(!child){child={name:part,path:parts.slice(0,position+1).join('/'),children:new Map(),audio:[]};node.children.set(part,child);}
   node=child;if(audioFormat(file.name))node.audio.push(index);
   if(position===parts.length-1)node.fileIndex=index;
  });
 });
 const audioOrder=new Map<string,number>();
 function orderChildren(node:Node){
  if(node.fileIndex!==undefined){
   const name=files[node.fileIndex].name;
   if(audioFormat(name))audioOrder.set(name,audioOrder.size);
   return;
  }
  const children=[...node.children.entries()].sort(([,a],[,b])=>{
   if(a.fileIndex===undefined&&b.fileIndex===undefined)return a.name.localeCompare(b.name,undefined,{numeric:true});
   if(a.fileIndex===undefined)return -1;
   if(b.fileIndex===undefined)return 1;
   const format=audioFormat(a.name);
   if(format&&format===audioFormat(b.name)&&manualOrder.has(orderGroup(node.path,format)))return a.fileIndex-b.fileIndex;
   return compareImportFiles(a.name,b.name);
  });
  if(manualOrder.has(orderGroup(node.path,null))){
   const folders=children.filter(([,child])=>audioFolder(child)).sort(([,a],[,b])=>a.audio[0]-b.audio[0]);
   let folderIndex=0;
   children.forEach((entry,index)=>{
    if(audioFolder(entry[1]))children[index]=folders[folderIndex++];
   });
  }
  node.children=new Map(children);
  for(const child of node.children.values())orderChildren(child);
 }
 orderChildren(root);
 return {root,audioOrder};
}

function SelectionBox({checked,partial,label,disabled,onChange}:{checked:boolean;partial:boolean;label:string;disabled:boolean;onChange:()=>void}){
 return <input ref={element=>{if(element)element.indeterminate=partial;}} type="checkbox" checked={checked} aria-label={label} disabled={disabled} onChange={onChange}/>;
}

function DiscChoice({label,value,discCount,allowNewDisc=false,disabled,onChange}:{label:string;value:number|undefined;discCount:number;allowNewDisc?:boolean;disabled:boolean;onChange:(disc:number)=>void}){
 const nextDisc=allowNewDisc&&discCount<maxMusicNumber?discCount+1:undefined;
 const limit=nextDisc??discCount;
 if(discCount>32)return <label className="import-disc-choice"><input key={value} type="number" min={1} max={limit}
  aria-label={label} placeholder={value===undefined?'Automatic / mixed':undefined} defaultValue={value} disabled={disabled} onBlur={event=>{
   const disc=event.currentTarget.valueAsNumber;
   if(Number.isInteger(disc)&&disc>=1&&disc<=limit)onChange(disc);
   else event.currentTarget.value=value?.toString()??'';
  }} onKeyDown={event=>{if(event.key==='Enter')event.currentTarget.blur();}}/></label>;
 const discs=Array.from({length:discCount},(_,i)=>i+1);
 return <label className="import-disc-choice"><select aria-label={label} disabled={disabled} value={value??''} onChange={event=>onChange(Number(event.target.value))}>
  <option value="" disabled>{allowNewDisc?'Automatic':'Auto / mixed'}</option>
  {discs.map(disc=><option key={disc} value={disc}>Disc {disc}</option>)}
  {nextDisc!==undefined&&<option value={nextDisc}>New disc ({nextDisc})</option>}
 </select></label>;
}

function TrackNumber({id,name,value,suggested,disabled,onCommit}:{
 id:string;
 name:string;
 value:number|null|undefined;
 suggested:number|undefined;
 disabled:boolean;
 onCommit:(number:number|null,keepFocus:boolean)=>void;
}){
 function commit(input:HTMLInputElement,keepFocus:boolean){
  const number=input.value===''?null:input.valueAsNumber;
  if(!input.checkValidity()||(number!==null&&!Number.isInteger(number))){
   if(keepFocus)input.reportValidity();
   else input.value=value?.toString()??'';
   return;
  }
  if(number!==(value??null))onCommit(number,keepFocus);
 }
 return <input id={id} type="number" min={1} max={maxMusicNumber} step={1}
  aria-label={`Track number for ${name}`}
  aria-description={suggested===undefined?undefined:`Suggested track number: ${suggested}`}
  placeholder={suggested?.toString()} defaultValue={value??''} disabled={disabled}
  onBlur={event=>commit(event.currentTarget,false)}
  onKeyDown={event=>{
   if(event.key!=='Enter')return;
   event.preventDefault();
   commit(event.currentTarget,true);
  }}/>;
}

type TreeProps={
 files:File[];
 tracks:Track[];
 disabled:boolean;
 onTracks:(tracks:Track[])=>void;
 onVisibility:(index:number,privateFile:boolean)=>void;
 onFileOrder:(group:string,indices:number[])=>void;
};
type Move={
 id:string;
 up?:()=>void;
 down?:()=>void;
 number?:(number:number|null,keepFocus:boolean)=>void;
};
type BranchProps=TreeProps&{
 node:Node;
 depth:number;
 selected:Map<string,Track>;
 sourceNumbers:Map<string,number>;
 discCount:number;
 canApplyBelow:(name:string)=>boolean;
 applyBelow:(name:string,disc:number)=>void;
 move?:Move;
};

function OrderButtons({move,name,disabled}:{move:Move;name:string;disabled:boolean}){
 return <div className="import-order-buttons">
  <button id={move.id+'-up'} type="button" disabled={disabled||!move.up} aria-label={`Move ${name} up`} onClick={move.up}>↑</button>
  <button id={move.id+'-down'} type="button" disabled={disabled||!move.down} aria-label={`Move ${name} down`} onClick={move.down}>↓</button>
 </div>;
}

function Branch(props:BranchProps){
 const {node,depth,files,tracks,selected,sourceNumbers,discCount,disabled,onTracks,onVisibility,onFileOrder,canApplyBelow,applyBelow,move}=props;
 const [open,setOpen]=useState(!node.path);
 const [page,setPage]=useState(0);
 const folderId=useId();
 const focusAfterMove=useRef<string|undefined>(undefined);
 React.useLayoutEffect(()=>{
  if(!focusAfterMove.current)return;
  document.getElementById(focusAfterMove.current)?.focus();
  focusAfterMove.current=undefined;
 },[page,files,tracks]);
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
 const indent={paddingInlineStart:`calc(.55rem + ${Math.min(depth,6)} * .85rem)`};
 if(node.fileIndex!==undefined){
  const index=node.fileIndex;
  const file=files[index];
  const playable=Boolean(audioFormat(file.name));
  const checked=selected.get(file.name);
  return <tr className={playable&&!checked?'import-tree-file playback-excluded':'import-tree-file'}>
   <td>{playable&&<SelectionBox checked={!!checked} partial={false} label={`Use for album playback: ${file.name}`} disabled={disabled} onChange={()=>choose(!checked)}/>}</td>
   <th scope="row" style={indent}>{node.name}</th>
   <td>{checked&&<DiscChoice label={`Disc for ${file.name}`} value={disc} discCount={discCount} allowNewDisc disabled={disabled} onChange={assign}/>}</td>
   <td>{checked&&move?.number&&<TrackNumber id={move.id+'-number'} name={file.name}
    value={checked.display_position} suggested={sourceNumbers.get(file.name)} disabled={disabled}
    onCommit={move.number}/>}</td>
   <td>{disc!==undefined&&canApplyBelow(file.name)&&<button type="button" disabled={disabled}
    aria-label={`Apply to below: Disc ${disc} from ${file.name}`}
    onClick={()=>applyBelow(file.name,disc)}>Apply to below</button>}</td>
   <td>{move&&<OrderButtons move={move} name={file.name} disabled={disabled}/>}</td>
   <td className="import-file-size">{bytes(file.bytes)}</td>
   <td><input type="checkbox" checked={file.visibility==='curators'} aria-label={`Curators only: ${file.name}`}
    disabled={disabled} onChange={event=>onVisibility(index,event.target.checked)}/></td>
  </tr>;
 }
 const children=open?[...node.children.values()]:[];
 const pageStart=page*pageSize;
 const formatAt=(child:Node|undefined)=>child?.fileIndex===undefined?undefined:audioFormat(files[child.fileIndex].name);
 const groups=new Map<string,number[]>();
 const folders:number[]=[];
 children.forEach((child,index)=>{
  if(audioFolder(child))folders.push(index);
  const format=formatAt(child);
  if(!format)return;
  const group=groups.get(format)??[];
  group.push(index);
  groups.set(format,group);
 });
 function swapControls(id:string,position:number,peers:number[],save:(target:number)=>void){
  const offset=peers.indexOf(position);
  function swap(target:number|undefined,direction:'up'|'down'){
   if(target===undefined)return;
   return ()=>{
    const nextPage=Math.floor(target/pageSize);
    const atEdge=direction==='up'?offset===1:offset===peers.length-2;
    if(nextPage!==page||atEdge){
     focusAfterMove.current=id+'-'+(atEdge?(direction==='up'?'down':'up'):direction);
    }
    if(nextPage!==page)setPage(nextPage);
    save(target);
   };
  }
  return {id,up:swap(peers[offset-1],'up'),down:swap(peers[offset+1],'down')};
 }
 function moveControls(child:Node,position:number):Move|undefined{
  const id=folderId+encodeURIComponent(child.path);
  if(audioFolder(child)&&folders.length>1){
   return swapControls(id,position,folders,target=>{
    const indices=folders.flatMap(index=>orderedAudio(children[index===position?target:index===target?position:index]));
    onFileOrder(orderGroup(node.path,null),indices);
   });
  }
  const format=formatAt(child);
  const track=selected.get(child.path);
  if(!format||!track)return;
  const group=groups.get(format)!;
  const checked=group.filter(index=>selected.has(children[index].path));

  function saveOrder(slots:number[],order:number[]){
   const bySlot=new Map(slots.map((slot,i)=>[slot,order[i]]));
   const indices=group.map(index=>children[bySlot.get(index)??index].fileIndex!);
   onFileOrder(orderGroup(node.path,format!),indices);
  }
  function number(display_position:number|null,keepFocus:boolean){
   const nextTracks=tracks.map(value=>value.file_name===child.path?{...value,display_position}:value);
   onTracks(nextTracks);
   if(display_position===null)return;
   const nextNumbers=automaticTrackNumbers(files,nextTracks);
   const fileIndex=(index:number)=>children[index].fileIndex!;
   const sourceOrder=[...group].sort((a,b)=>fileIndex(a)-fileIndex(b));
   const discPeers=sourceOrder.filter(index=>{
    const peer=selected.get(children[index].path);
    return peer&&trackDisc(peer)===trackDisc(track!);
   });
   const nextOrder=discPeers.filter(index=>index!==position);
   const nextHigher=nextOrder.findIndex(index=>{
    const name=children[index].path;
    const peerNumber=selected.get(name)?.display_position??nextNumbers.get(name);
    return peerNumber!==undefined&&(peerNumber>display_position||peerNumber===display_position&&fileIndex(index)>fileIndex(position));
   });
   const destination=nextHigher===-1?nextOrder.length:nextHigher;
   if(discPeers[destination]===position)return;
   nextOrder.splice(destination,0,position);
   if(keepFocus){
    focusAfterMove.current=id+'-number';
    setPage(Math.floor(group[sourceOrder.indexOf(discPeers[destination])]/pageSize));
   }
   onFileOrder(orderGroup(node.path,format!),nextOrder.map(fileIndex));
  }
  return {...swapControls(id,position,checked,target=>saveOrder([position,target],[target,position])),number};
 }

 return <>
  <tr className="import-folder-row">
   <td>{!!node.audio.length&&<SelectionBox checked={included.length===node.audio.length} partial={included.length>0&&included.length<node.audio.length}
    label={`Use folder for album playback: ${node.path||'All files'}`} disabled={disabled} onChange={()=>choose(included.length!==node.audio.length)}/>}</td>
   <th scope="row" style={indent}><button type="button" className="import-folder-toggle" aria-expanded={open}
    onClick={()=>setOpen(value=>!value)}>{open?'▾':'▸'} {node.name}</button>
    {!!node.audio.length&&<small>{included.length} / {node.audio.length} selected</small>}</th>
   <td>{!!included.length&&<DiscChoice label={`Disc for folder ${node.path||'All files'}`} value={disc} discCount={discCount} disabled={disabled} onChange={assign}/>}</td>
   <td colSpan={2}></td>
   <td>{move&&<OrderButtons move={move} name={`folder ${node.path}`} disabled={disabled}/>}</td>
   <td colSpan={2}></td>
  </tr>
  {children.slice(pageStart,pageStart+pageSize).map((child,i)=>{
   const position=pageStart+i;
   const format=formatAt(child);
   return <React.Fragment key={child.path}>
    {groups.size>1&&format&&(i===0||formatAt(children[position-1])!==format)&&
     <tr className="import-format-heading"><td></td><td colSpan={7} style={indent}>{format.toUpperCase()}</td></tr>}
    <Branch {...props} node={child} depth={depth+1} move={moveControls(child,position)}/>
   </React.Fragment>;
  })}
  {children.length>pageSize&&<tr className="import-file-pages"><td colSpan={8} style={indent}>
   <button disabled={!page} onClick={()=>setPage(value=>value-1)}>Previous entries</button>
   <span>Page {page+1} of {Math.ceil(children.length/pageSize)}</span>
   <button disabled={(page+1)*pageSize>=children.length} onClick={()=>setPage(value=>value+1)}>Next entries</button>
  </td></tr>}
 </>;
}

const defaultOrder:readonly string[]=[];

export default function ImportFileTree({layout,reviewed,manualGroups=defaultOrder,...props}:{
 layout:S['MusicImportLayout'];
 reviewed:boolean;
 manualGroups?:readonly string[];
}&TreeProps){
 const {files,tracks,disabled,onTracks}=props;
 const [open,setOpen]=useState(false);
 const orderedTree=React.useMemo(()=>open?tree(files,manualGroups):undefined,[files,open,manualGroups]);
 const audioOrder=orderedTree?.audioOrder;
 const lastSelectedPosition=tracks.reduce((last,track)=>Math.max(last,audioOrder?.get(track.file_name)??-1),-1);
 function canApplyBelow(name:string){
  const position=audioOrder?.get(name);
  return position!==undefined&&position<lastSelectedPosition;
 }
 function applyBelow(name:string,disc:number){
  const start=audioOrder?.get(name);
  if(start===undefined)return;
  onTracks(tracks.map(track=>(audioOrder?.get(track.file_name)??-1)>=start
   ?{...track,disc_position:disc}:track));
 }
 const selected=React.useMemo(()=>new Map(tracks.map(track=>[track.file_name,track])),[tracks]);
 const sourceNumbers=React.useMemo(()=>open?automaticTrackNumbers(files,tracks):new Map<string,number>(),[files,open,tracks]);
 const highestDisc=tracks.reduce((highest,track)=>Math.max(highest,track.disc_position??1),1);
 const [extraDiscs,setExtraDiscs]=useState<number>();
 const discCount=Math.max(highestDisc,extraDiscs??1);
 const [countInput,setCountInput]=useState<string>();
 const targetCount=Number(countInput??discCount);
 const validCount=Number.isInteger(targetCount)&&targetCount>=1&&targetCount<=maxMusicNumber;
 function applyDiscCount(){
  if(!validCount)return;
  setExtraDiscs(targetCount);
  setCountInput(undefined);
  if(tracks.length)onTracks(tracks.map(track=>({...track,disc_position:Math.min(track.disc_position??1,targetCount)})));
 }
 const formats=React.useMemo(()=>['flac','wav','mp3'].filter(format=>files.some(file=>audioFormat(file.name)===format)),[files]);
 function selectFormat(format:string){
  const guesses=new Map(layout.tracks.map(track=>[track.file_name,track.disc_position]));
  onTracks(files.filter(file=>audioFormat(file.name)===format).map(file=>selected.get(file.name)??{file_name:file.name,disc_position:guesses.get(file.name)??pathDisc(file.name)}));
 }
 return <details onToggle={event=>setOpen(event.currentTarget.open)}><summary>Files and album playback · {tracks.length} tracks {reviewed?'selected':'suggested'}{layout.needs_attention&&!reviewed?' · Needs attention':''}</summary>
  {open&&<><p>All {files.length} originals will be preserved. Check the tracks you want for playback. Moving folders or entering track numbers sets playback order on the affected discs. Automatic numbers fill the open positions.</p>
   {layout.notices.map(notice=><p key={notice} className="notice">{notice}</p>)}
   <div className="import-format-actions" role="group" aria-label="Choose playback format">{formats.length>1&&formats.map(format=><button key={format} disabled={disabled} onClick={()=>selectFormat(format)}>Select {format.toUpperCase()}</button>)}{layout.needs_attention&&!reviewed&&<button disabled={disabled||!tracks.length} onClick={()=>onTracks(tracks)}>Confirm playback selection</button>}</div>
   <div className="import-disc-setup">
    <label>Number of discs<input type="number" min={1} max={maxMusicNumber} step={1} disabled={disabled} value={countInput??discCount}
     onChange={event=>setCountInput(event.currentTarget.value)} onKeyDown={event=>{if(event.key==='Enter'&&countInput!==undefined)applyDiscCount();}}/></label>
    <button disabled={disabled||!validCount||countInput===undefined} onClick={applyDiscCount}>Apply disc count</button>
    <small>{validCount&&targetCount<highestDisc?`Tracks on higher discs will move to Disc ${targetCount}.`:countInput!==undefined&&tracks.some(track=>track.disc_position==null)?'Tracks with automatic numbering will start on Disc 1.':'Choose a disc on its first track, then use Apply to below for later checked tracks.'}</small>
   </div>
   {!tracks.length&&<p role="alert">Select at least one audio file for album playback.</p>}
   {orderedTree&&<div className="import-file-scroll" role="region" aria-label="Album file table" tabIndex={0}>
    <table className="import-file-tree" aria-label="Album files">
     <colgroup><col className="import-play-column"/><col/><col className="import-disc-column"/><col className="import-track-column"/><col className="import-apply-column"/>
      <col className="import-order-column"/><col className="import-size-column"/><col className="import-privacy-column"/></colgroup>
     <thead><tr><th scope="col">Play</th><th scope="col">File</th><th scope="col">Disc</th><th scope="col">Track no.</th><th scope="col">Assign below</th>
      <th scope="col">Order</th><th scope="col">Size</th><th scope="col">Curators only</th></tr></thead>
     <tbody className="striped-rows"><Branch {...props} node={orderedTree.root} depth={0}
      selected={selected} sourceNumbers={sourceNumbers} discCount={discCount} canApplyBelow={canApplyBelow} applyBelow={applyBelow}/></tbody>
    </table>
   </div>}
  </>}
 </details>;
}
