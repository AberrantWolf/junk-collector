import {profileLabel} from './audio-profiles';
import InfoHelp from './info-help';
import {presentationLabel} from './presentation-labels';
import React,{useEffect,useState,api,requestErrorMessage,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';
import {bytes} from './work-progress';
type S=components['schemas'];export type Summary=S['BulkLibrarySummary'];
export default function BulkReview({id,onOpen,onChanged,onSessionExpired}:{id:string;onOpen:(item:string,collection:string,kind:S['BulkLibraryKind'],profile?:S['ConversionProfile'])=>void;onChanged?:()=>void;onSessionExpired:()=>void}){
 const [summary,setSummary]=useState<Summary>(),[members,setMembers]=useState<S['BulkLibraryMemberView'][]>();
 const [cursor,setCursor]=useState<string|null>(null),[error,setError]=useState(''),[loadError,setLoadError]=useState(''),[busy,setBusy]=useState(false),[tick,setTick]=useState(0);
 const [pages,setPages]=useState(1),[pageTrail,setPageTrail]=useState<string[]>([]);
 function fail(e:unknown,report=setError){if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else report(requestErrorMessage(e));}
 useEffect(()=>{
  const controller=new AbortController();
  let timer:ReturnType<typeof setTimeout>;
  async function load(){
   try{
    const summary=await api<Summary>(`/bulk-library/${id}`,{signal:controller.signal});
    const rows:S['BulkLibraryMemberView'][]=[];
    let next=summary.plans_during_admission?pageTrail.at(-1)??null:null;
    const count=summary.plans_during_admission?1:pages;
    for(let i=0;i<count;i++){
     const page:{items:S['BulkLibraryMemberView'][];next_cursor:string|null}=await api(`/bulk-library/${id}/members?limit=50${next?'&cursor='+next:''}`,{signal:controller.signal});
     rows.push(...page.items);
     next=page.next_cursor;
     if(!next)break;
    }
    if(controller.signal.aborted)return;
    setSummary(summary);setMembers(rows);setCursor(next);setLoadError('');
    if(summary.plans_during_admission&&(summary.state==='completed'||summary.state==='cancelled'))return;
   }catch(e){if(!controller.signal.aborted)fail(e,setLoadError);}
   if(!controller.signal.aborted)timer=setTimeout(load,2000);
  }
  void load();
  return()=>{controller.abort();clearTimeout(timer);};
 },[id,pages,pageTrail,tick]);
 function movePage(next?:string){
  setMembers(undefined);setCursor(null);
  setPageTrail(trail=>next?[...trail,next]:trail.slice(0,-1));
 }
 async function act(action:'start'|'cancel'|'retry'){setBusy(true);setError('');try{setSummary(await api<Summary>(`/bulk-library/${id}/${action}`,{method:'POST'}));setTick(v=>v+1);onChanged?.();}catch(e){fail(e);}finally{setBusy(false);}}
 return <section className="care-review bulk-review" aria-label="Review batch" tabIndex={-1}>{(error||loadError)&&<p role="alert" className="notice error">{error||loadError}</p>}{summary?<><h2>{summary.kind==='metadata'?'Metadata refresh':summary.kind==='accurate_rip'?'AccurateRip checks':profileLabel(summary.profile)+' preparation'} · {presentationLabel(summary.state)}</h2><p>{summary.selected} selected albums · {summary.excluded} excluded · {summary.succeeded} successful · {summary.failed} need attention · {summary.pending} pending</p>{summary.kind==='music_preparation'&&<><p>{summary.plans_during_admission&&summary.state==='accepted'&&summary.tracks===0?'Planning albums in the background…':<>{summary.tracks} {summary.plans_during_admission?'tracks planned':'selected tracks'} · {summary.already_ready} already ready</>}</p>{!summary.plans_during_admission&&<p className="muted">Original files: {bytes(summary.source_bytes)} <InfoHelp label="original file size">This is the size of the original files. The prepared files may have a different size.</InfoHelp></p>}</>}{summary.state==='preview'&&<p>This batch uses the selection shown here, even if you change filters.</p>}{summary.admission_error&&<p className="notice error" role="alert">Could not start some tasks: {summary.admission_error}.</p>}<div className="care-actions">{summary.kind==='music_preparation'&&(!summary.plans_during_admission||summary.state==='completed'&&summary.failed===0)&&summary.tracks>0&&summary.already_ready===summary.tracks&&<p role="status">All selected tracks are already ready as {profileLabel(summary.profile)}.</p>}{summary.can_start&&(summary.kind!=='music_preparation'||summary.tracks>summary.already_ready)&&<button className="primary" disabled={busy} onClick={()=>void act('start')}>Start {summary.kind==='metadata'?'metadata refresh':summary.kind==='accurate_rip'?'AccurateRip checks':profileLabel(summary.profile)+' preparation'}</button>}{summary.can_cancel&&<button disabled={busy} onClick={()=>void act('cancel')}>Cancel remaining work</button>}{summary.can_retry&&<button disabled={busy} onClick={()=>void act('retry')}>{summary.admission_error?'Retry unfinished tasks':'Retry unfinished tasks'}</button>}</div><div className="care-rows">{members?.map(m=><div className="care-row" key={m.position}>{m.artwork_asset_id?<img loading="lazy" src={`/api/v1/collections/${m.collection_id}/assets/${m.artwork_asset_id}/thumbnail`} alt={`${m.title} artwork`}/>:<span className="care-cover-empty">◇</span>}<div className="row-text"><strong>{m.title}</strong><small>{summary.plans_during_admission&&m.state==='ready'?(m.job_id?'Waiting to retry':'Waiting to plan'):presentationLabel(m.state)}{m.tracks?` · ${m.tracks} tracks · ${m.already_ready} already ready`:''}</small>{m.reason&&<p className="notice">{m.reason}</p>}</div><button onClick={()=>onOpen(m.item_id,m.collection_id,summary.kind,summary.profile)}>Open {summary.kind==='metadata'?'metadata review':summary.kind==='accurate_rip'?'disc results':'album preparation'}</button></div>)}</div>{summary.plans_during_admission&&pageTrail.length>0&&<button disabled={busy||!members} onClick={()=>movePage()}>Previous album results</button>}{cursor&&<button disabled={busy} onClick={()=>summary.plans_during_admission?movePage(cursor):setPages(v=>v+1)}>{summary.plans_during_admission?'Next album results':'More album results'}</button>}{!members&&<p role="status">Loading album results…</p>}</>:<p role="status">Loading batch…</p>}</section>;
}
