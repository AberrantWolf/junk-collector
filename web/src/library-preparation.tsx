import MoreButton from './more-button';
import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import MusicTracks from './music-tracks';
import BulkReview,{type Summary} from './bulk-library-review';
import {PreparationFormat,conversionProfile,preparationPreset,preparationQuery,profileLabel,type PreparationPreset} from './audio-profiles';

type S=components['schemas'];
type Entry=S['PreparationCareEntry'];
type Page=S['PreparationCarePage'];
type Selection={selected:string[];all:boolean;exclusions:string[];sources:Record<string,string[]>};
const emptySelection:Selection={selected:[],all:false,exclusions:[],sources:{}};
const endpoint='/library-care/music-preparation';

export type Props={pages:number;onMore:()=>void;userId:string;collection:string;search:string;attention:boolean;preset:PreparationPreset;itemId:string;batchId:string;onQuery:(v:{item?:string;batch?:string;attention?:boolean;collection?:string;preparationPreset?:PreparationPreset})=>void;onMetadata:(item:string,collection:string)=>void;onChanged:()=>void;onSessionExpired:()=>void};

export default function Preparation({pages,onMore,userId,collection,search,attention,preset,itemId,batchId,onQuery,onMetadata,onChanged,onSessionExpired}:Props){
 const scope=JSON.stringify([userId,collection,search,attention,preset]);
 const draftKey=`collector:preparation-selection:${userId}`;
 const [draft,setDraft]=useState<{key:string;value:Selection}>();
 const selection=draft?.key===scope?draft.value:emptySelection;
 const [list,setList]=useState<{scope:string;rows:Entry[];cursor?:string|null;state:Page['summary_state']}>();
 const [item,setItem]=useState<S['PresentedItem']>();
 const [error,setError]=useState('');
 const [listError,setListError]=useState('');
 const [loading,setLoading]=useState(true);
 const [previewing,setPreviewing]=useState(false);
 const [reload,setReload]=useState(0);
 const lastRevision=useRef('');
 const previewRequest=useRef<AbortController|null>(null);
 const previewReceipt=useRef<{body:string;key:string}|undefined>(undefined);
 const callbacks=useRef({onSessionExpired,onChanged,onQuery});callbacks.current={onSessionExpired,onChanged,onQuery};
 const profile=conversionProfile(preset);
 const label=profileLabel(profile);
 const rows=list?.scope===scope?list.rows:[];
 const current=list?.scope===scope?list:undefined;

 function fail(e:unknown){
  if(isApiRequestError(e)&&e.code==='unauthenticated')callbacks.current.onSessionExpired();
  else setError(requestErrorMessage(e));
 }
 function updateSelection(change:(value:Selection)=>Selection){
  setDraft(current=>({key:scope,value:change(current?.key===scope?current.value:emptySelection)}));
 }
 useEffect(()=>{
  try{
   const saved=JSON.parse(sessionStorage.getItem(draftKey)||'null');
   setDraft(saved?.key===scope?saved:{key:scope,value:{...emptySelection,sources:saved?.value?.sources||{}}});
  }catch{setDraft({key:scope,value:emptySelection});}
 },[scope,draftKey]);
 useEffect(()=>()=>previewRequest.current?.abort(),[scope,itemId,batchId]);
 useEffect(()=>{
  if(draft?.key===scope)try{sessionStorage.setItem(draftKey,JSON.stringify(draft));}catch{}
 },[draft,draftKey,scope]);

 useEffect(()=>{
  const controller=new AbortController();
  const {signal}=controller;
  let timer:ReturnType<typeof setTimeout>;
  let loaded=false;
  async function load(){
   let next:string|null=null;
   let state:Page['summary_state']='pending';
   let revision='';
   const found:Entry[]=[];
   for(let n=0;n<pages;n++){
    const params=new URLSearchParams(preparationQuery(preset));
    params.set('search',search);params.set('needs_attention',String(attention));params.set('limit','50');
    if(collection)params.set('collection_id',collection);
    if(next)params.set('cursor',next);
    const page:Page=await api(`${endpoint}?${params}`,{signal});
    signal.throwIfAborted();
    found.push(...page.items);next=page.next_cursor??null;state=page.summary_state;revision=page.catalog_revision;
    if(!next)break;
   }
   setList({scope,rows:found,cursor:next,state});
   lastRevision.current=revision;
   loaded=true;
  }
  async function poll(){
   try{
    if(!loaded||await api<string>(`${endpoint}/revision`,{signal})!==lastRevision.current){
     await load();
    }
    if(!signal.aborted)setListError('');
   }catch(e){
    if(signal.aborted)return;
    loaded=false;
    if(isApiRequestError(e)&&e.code==='unauthenticated')callbacks.current.onSessionExpired();
    else if(!isApiRequestError(e)||e.status!==409)setListError(requestErrorMessage(e));
   }finally{
    if(!signal.aborted){setLoading(false);timer=setTimeout(poll,1500);}
   }
  }
  setLoading(true);setError('');setListError('');void poll();
  return()=>{controller.abort();clearTimeout(timer);};
 },[scope,reload,pages]);

 useEffect(()=>{
  if(!itemId&&!batchId&&!loading&&rows.length){
   if(history.state?.focus)document.querySelector<HTMLElement>(`[data-item-id="${CSS.escape(history.state.focus)}"]`)?.focus({preventScroll:true});
   window.scrollTo(0,history.state?.scroll||0);
  }
 },[itemId,batchId,loading,scope]);
 useEffect(()=>{
  if(!itemId)return;
  const controller=new AbortController();
  api<S['PresentedItem']>(`/items/${itemId}`,{signal:controller.signal}).then(value=>{
   if(!controller.signal.aborted)setItem(value);
  }).catch(e=>{if(!controller.signal.aborted)fail(e);});
  return()=>controller.abort();
 },[itemId,reload]);
 useEffect(()=>{
  if(itemId||batchId)document.querySelector<HTMLElement>('.care-review')?.focus({preventScroll:true});
 },[itemId,batchId]);

 async function preview(){
  const controller=new AbortController();previewRequest.current=controller;
  setPreviewing(true);setError('');
  const query={collection_id:collection||null,search,needs_attention:attention,reason:null,cursor:null,limit:null};
  const body=JSON.stringify({kind:'music_preparation',profile,selection:selection.all?{selection:'all_matching',query,exclusions:selection.exclusions}:{selection:'explicit',items:selection.selected},sources:selection.sources});
  if(previewReceipt.current?.body!==body)previewReceipt.current={body,key:crypto.randomUUID()};
  try{
   const result=await api<Summary>('/bulk-library/preview',{method:'POST',signal:controller.signal,headers:{'Idempotency-Key':previewReceipt.current.key},body});
   if(!controller.signal.aborted){previewReceipt.current=undefined;callbacks.current.onQuery({batch:result.id,item:''});}
  }catch(e){if(!controller.signal.aborted)fail(e);}
  finally{if(previewRequest.current===controller)setPreviewing(false);}
 }
 function toggle(id:string,on:boolean){
  updateSelection(value=>value.all?{...value,exclusions:on?value.exclusions.filter(x=>x!==id):[...value.exclusions,id]}:{...value,selected:on?[...value.selected,id]:value.selected.filter(x=>x!==id)});
 }
 return <section className={`care-workspace ${itemId||batchId?'care-review-open':''}`} aria-label="Prepare audio workspace">
  <div className="care-list">
   {!itemId&&<PreparationFormat value={preset} disabled={previewing} onChange={preparationPreset=>onQuery({preparationPreset,item:'',batch:''})}/>}
   <label>Show<select aria-label="Show" value={attention?'missing':'all'} disabled={previewing} onChange={e=>onQuery({attention:e.target.value==='missing',item:'',batch:''})}>
    <option value="missing">Albums needing {profile.codec==='opus'?'Opus':'FLAC'}</option><option value="all">All music albums</option>
   </select></label>
   <p>Prepare {label} files for your albums.</p>
   {(error||listError)&&<p role="alert" className="notice error">{error||listError} <button onClick={()=>setReload(v=>v+1)}>Retry loading albums</button></p>}
   <div className="care-selection">
    <strong>{selection.all?'All matching results':`${selection.selected.length} albums selected`}{selection.all&&selection.exclusions.length?` · ${selection.exclusions.length} excluded`:''}</strong>
    <button disabled={previewing||!rows.length||attention&&current?.state!=='ready'} onClick={()=>updateSelection(v=>({...v,all:true,selected:[],exclusions:[]}))}>Select all matching results</button>
    <button disabled={previewing||loading||selection.all&&attention&&current?.state!=='ready'||!selection.all&&!selection.selected.length} className="primary" onClick={()=>void preview()}>Preview {label} preparation</button>
    <button disabled={previewing} onClick={()=>updateSelection(v=>({...v,selected:[],all:false,exclusions:[]}))}>Clear selection</button>
   </div>
   {rows.map(({item:i,readiness})=><div className="care-selectable-row" key={i.id}>
    <input type="checkbox" aria-label={`Select ${i.title}`} checked={selection.all?!selection.exclusions.includes(i.id):selection.selected.includes(i.id)} disabled={previewing} onChange={e=>toggle(i.id,e.target.checked)}/>
    <button className="care-row" data-item-id={i.id} onClick={()=>onQuery({item:i.id,batch:''})}>
     {i.artwork?<img loading="lazy" src={`/api/v1/collections/${i.collection_id}/assets/${i.artwork.id}/thumbnail`} alt={i.artwork.alt_text||`${i.title} artwork`}/>:<span className="care-cover-empty">◇</span>}
     <span><strong>{i.title}</strong>{readiness?<small>{readiness.tracks} tracks · {readiness.ready} ready{readiness.preparing>0?` · ${readiness.preparing} preparing`:''}</small>:<small>Updating readiness…</small>}</span>
    </button>
   </div>)}
   {!rows.length&&!loading&&!error&&!listError&&current?.state==='ready'&&<p>No albums match these preparation filters.</p>}
   {current?.cursor&&<MoreButton loading={loading} loadingText="Loading albums…" disabled={previewing} onClick={onMore}>More albums</MoreButton>}
   {loading&&<p role="status">Loading preparation…</p>}
   {!loading&&current&&current.state!=='ready'&&<p role="status">{current.state==='retrying'?'Retrying preparation summaries…':'Updating preparation summaries…'}</p>}
  </div>
  {batchId?<div>
   <button className="care-return" onClick={()=>onQuery({batch:''})}>Back to preparation results</button>
   <BulkReview key={batchId} id={batchId} onOpen={(item,collection,_kind,profile)=>onQuery({item,collection,batch:'',preparationPreset:preparationPreset(profile?.codec==='opus'?String(profile.bitrate_kbps):null)})} onChanged={onChanged} onSessionExpired={onSessionExpired}/>
  </div>:itemId&&<section className="care-review" tabIndex={-1} aria-label="Album preparation">
   <button className="care-return" onClick={()=>onQuery({item:''})}>Back to preparation results</button>
   {item?.id===itemId?<><h2>{item.title}</h2><MusicTracks key={item.id} item={item} canPrepare={item.content_permissions.can_edit}
    preparationPreset={preset} onPreparationPreset={preparationPreset=>onQuery({preparationPreset})}
    preparationSources={selection.sources[item.id]} onPreparationSources={sources=>updateSelection(v=>({...v,sources:{...v.sources,[item.id]:sources}}))}
    onPrepared={saved=>{setItem(saved);callbacks.current.onChanged();}} onOpenMetadata={()=>onMetadata(item.id,item.collection_id)} onSessionExpired={onSessionExpired}/></>:<p role="status">Opening album…</p>}
  </section>}
 </section>;
}
