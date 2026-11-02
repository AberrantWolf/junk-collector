import React,{useEffect,useRef,useState,api,requestErrorMessage,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';
type Item=components['schemas']['PresentedItem'];
type Assignment=components['schemas']['DigitalTrackAssignment'];
type Mapping={key:string;medium:number;position:number;recording_id:string;score:number;duration_delta_ms:number|null};
type Candidate={release:{id:string;title:string;artist:string;disambiguation:string|null;date:string|null;country:string|null;media:{position:number|null;format:string|null;tracks:{position:number;title:string;duration_ms?:number|null}[]}[]};mappings:Mapping[];local_tracks:number;release_tracks:number;complete_tracklist:boolean;ambiguous_mapping:boolean;duration_conflicts:number;unmatched:string[]};
type Result={inventory_revision:number;tracks:{source:string;file:string;name:string;frames:number;sample_rate:number}[];matches:Candidate[];truncated:boolean;artwork:{release_id:string;snapshot_id:string;image_id:string}[];artwork_error:string|null};
type Work={stale:boolean;job:{id:string;state:string;phase:string;error:string|null;retryable:boolean};result:Result|null};
type Status={available:boolean;jobs:Work[]};
const active=new Set(['queued','running','cancel_requested']);
export default function DigitalAlbumMatching({userId,item,metadataRevision,disabled,onChoose,onError}:{userId:string;item:Item;metadataRevision:number;disabled:boolean;onChoose:(job:string,release:string,assignments?:Assignment[])=>Promise<void>;onError:(error:unknown)=>void}){
 const [status,setStatus]=useState<Status>();const [error,setError]=useState('');const [loadError,setLoadError]=useState('');const [busy,setBusy]=useState(false);const [reload,setReload]=useState(0);
 const lifetime=useRef(new AbortController());const callbacks=useRef({onChoose,onError});callbacks.current={onChoose,onError};
 const owns=(controller:AbortController)=>lifetime.current===controller&&!controller.signal.aborted;
 useEffect(()=>{const controller=new AbortController();lifetime.current=controller;setStatus(undefined);setBusy(false);setError('');setLoadError('');let timer:ReturnType<typeof setTimeout>|undefined;
  async function load(){try{const next=await api<Status>(`/items/${item.id}/digital-album-matches`,{signal:controller.signal});if(controller.signal.aborted)return;setStatus(next);setLoadError('');if(next.jobs.some(w=>active.has(w.job.state)))timer=setTimeout(load,1000);}catch(e){if(!controller.signal.aborted){setLoadError(requestErrorMessage(e));callbacks.current.onError(e);timer=setTimeout(load,3000);}}}
  void load();return()=>{controller.abort();if(timer)clearTimeout(timer);};
 },[userId,item.id,item.revision,metadataRevision,reload]);
 const pending=status?.jobs.find(w=>active.has(w.job.state));const latest=status?.jobs[0];const completed=status?.jobs.find(w=>w.job.state==='completed'&&w.result);const inventoryChanged=!!completed?.result&&(completed.stale!==false||completed.result.inventory_revision!==item.revision);
 async function start(){
  const controller=lifetime.current;setBusy(true);setError('');const storage=`collector.digital-match.${userId}.${item.id}`;
  try{
   let saved:{key:string;expected_revision:number}|undefined;
   try{saved=JSON.parse(localStorage.getItem(storage)||'null')||undefined;}catch{}
   const recovering=!!saved;
   const fresh=()=>{const receipt={key:crypto.randomUUID(),expected_revision:item.revision};localStorage.setItem(storage,JSON.stringify(receipt));return receipt;};
   const submit=(receipt:{key:string;expected_revision:number})=>api<{job_id:string}>(`/items/${item.id}/digital-album-matches`,{method:'POST',headers:{'Idempotency-Key':receipt.key},body:JSON.stringify({expected_revision:receipt.expected_revision}),signal:controller.signal});
   const accepted=await submit(saved||fresh());
   if(!owns(controller))return;
   // Acknowledge a lost response before issuing the user's new search intent.
   if(recovering&&status?.jobs.some(work=>work.job.id===accepted.job_id&&!active.has(work.job.state))){
    await submit(fresh());if(!owns(controller))return;
   }
   localStorage.removeItem(storage);setReload(v=>v+1);
  }catch(e){if(owns(controller)){if(isApiRequestError(e)&&e.status>=400&&e.status<500&&e.code!=='command_in_progress')localStorage.removeItem(storage);setError(requestErrorMessage(e));callbacks.current.onError(e);}}
  finally{if(owns(controller))setBusy(false);}
 }
 async function jobAction(job:string,method:'POST'|'DELETE'){const controller=lifetime.current;setError('');setBusy(true);try{await api(`/jobs/${job}${method==='POST'?'/retry':''}`,{method,signal:controller.signal});if(owns(controller))setReload(v=>v+1);}catch(e){if(owns(controller))setError(requestErrorMessage(e));}finally{if(owns(controller))setBusy(false);}}
 if(!item.captures.some(c=>c.representations.some(r=>r.format==='original-file-audio-package')))return null;
 return <section aria-label="Digital album matching" className="digital-album-matching"><h4>Find matching digital albums</h4><p>Identify recordings from the audio when tags are missing. This sends fingerprints and track durations to AcoustID, then finds MusicBrainz releases. Audio files and purchase details stay on this server.</p>
 {status&&!status.available&&<p className="notice">Configure the server’s AcoustID application key and MusicBrainz contact to enable matching.</p>}
 <button disabled={disabled||busy||!!pending||!status?.available} onClick={()=>void start()}>Find matching albums</button>
 {pending&&<div role="status"><p>{pending.job.phase.replaceAll('_',' ')}…</p><button disabled={busy} onClick={()=>void jobAction(pending.job.id,'DELETE')}>Cancel album matching</button></div>}
 {latest?.job.error&&<p role="alert">{latest.job.error}</p>}{latest?.job.retryable&&!latest.stale&&!pending&&<button disabled={busy||disabled} onClick={()=>void jobAction(latest.job.id,'POST')}>Retry album matching</button>}{loadError&&<p role="alert">{loadError}</p>}{error&&<p role="alert">{error}</p>}
 {completed?.result&&<><p>These are identity suggestions, not catalog verification. Review the track mapping before applying a release.</p>{inventoryChanged&&<p className="notice">Album inventory or metadata changed. Find matching albums again before applying a release.</p>}{completed.result.truncated&&<p className="notice">The search reached its candidate budget. These results are incomplete.</p>}{completed.result.artwork_error&&<p role="alert">Artwork needs attention: {completed.result.artwork_error}</p>}{!completed.result.matches.length&&<p>No album candidates are available to review. Try a manual MusicBrainz search below.</p>}
 {completed.result.matches.map(candidate=><AlbumCandidate key={`${completed.job.id}:${candidate.release.id}`} candidate={candidate} result={completed.result!} item={item} stale={inventoryChanged} disabled={disabled||busy||!!pending} onChoose={assignments=>callbacks.current.onChoose(completed.job.id,candidate.release.id,assignments)}/> )}</>}

 </section>;
}

function duration(ms:number|null|undefined){return ms==null?'duration unknown':`${Math.floor(ms/60000)}:${String(Math.floor(ms/1000)%60).padStart(2,'0')}`;}
function AlbumCandidate({candidate,result,item,stale,disabled,onChoose}:{candidate:Candidate;result:Result;item:Item;stale:boolean;disabled:boolean;onChoose:(assignments?:Assignment[])=>Promise<void>}){
 const automatic=candidate.complete_tracklist&&!candidate.ambiguous_mapping&&!candidate.duration_conflicts&&candidate.mappings.length===candidate.local_tracks&&candidate.release_tracks===candidate.local_tracks;
 const [editing,setEditing]=useState(false);const [confirmed,setConfirmed]=useState(false);
 const [draft,setDraft]=useState<Record<string,string>>(()=>Object.fromEntries(candidate.mappings.map(m=>[m.key,`${m.medium}:${m.position}`])));
 const slots=candidate.release.media.flatMap(m=>m.position==null?[]:m.tracks.map(t=>({...t,medium:m.position!,value:`${m.position}:${t.position}`}))).sort((a,b)=>a.medium-b.medium||a.position-b.position);
 const files=[...result.tracks].sort((a,b)=>a.name.localeCompare(b.name,undefined,{numeric:true})||`${a.source}:${a.file}`.localeCompare(`${b.source}:${b.file}`));
 const canMap=candidate.complete_tracklist&&slots.length===files.length&&candidate.release_tracks===files.length;
 const values=files.map(t=>draft[`${t.source}:${t.file}`]||'');
 const complete=canMap&&values.every(v=>slots.some(s=>s.value===v))&&new Set(values).size===files.length;
 const canApply=!stale&&(editing?complete&&confirmed:automatic);
 const cover=result.artwork.find(a=>a.release_id===candidate.release.id);
 function update(next:Record<string,string>){setDraft(next);setConfirmed(false);}
 const assignments=()=>files.map(t=>{const slot=slots.find(s=>s.value===draft[`${t.source}:${t.file}`])!;return {key:`${t.source}:${t.file}`,medium:slot.medium,position:slot.position};});
 return <article className="candidate"><div className="digital-match-cover">{cover?<img loading="lazy" src={`/api/v1/items/${item.id}/metadata/artwork/${cover.snapshot_id}/${encodeURIComponent(cover.image_id)}/thumbnail`} alt={`Cover of ${candidate.release.title}`}/>:<span role="img" aria-label="No cover preview">♫</span>}</div><div>
 <strong>{candidate.release.title}</strong><p>{candidate.release.artist}</p><p>{[candidate.release.disambiguation,candidate.release.date,candidate.release.country].filter(Boolean).join(' · ')}</p>
 <p>{candidate.mappings.length} of {candidate.local_tracks} files associated by audio recognition · {candidate.release_tracks} release tracks{candidate.ambiguous_mapping?' · ambiguous track assignment':''}{candidate.duration_conflicts?' · conflicting durations':''}</p>
 {candidate.mappings.length<candidate.local_tracks&&<p className="notice">Partial album evidence: the remaining files have no recording association to this release.</p>}
 <p>Recognition can be incomplete. Confirming a mapping applies the album and every mapped track title; it does not verify the audio.</p>
 <details><summary>Review proposed track mapping</summary><p>AcoustID scores describe recording associations, not the probability that the whole album is correct.</p><ul>{[...candidate.mappings].sort((a,b)=>a.medium-b.medium||a.position-b.position).map(m=>{const track=slots.find(t=>t.medium===m.medium&&t.position===m.position);return <li key={m.key}>{result.tracks.find(t=>`${t.source}:${t.file}`===m.key)?.name||`File ${m.key.slice(-8)}`} → disc {m.medium}, track {m.position}: {track?.title||'Untitled'}{m.duration_delta_ms!=null?` (duration difference ${(m.duration_delta_ms/1000).toFixed(1)} s)`:''} · AcoustID score {m.score.toFixed(2)}</li>;})}</ul></details>
 {!stale&&canMap&&<button disabled={disabled} onClick={()=>{setEditing(v=>!v);setConfirmed(false);}}>{editing?'Close mapping editor':'Edit track mapping'}</button>}
 {!automatic&&!editing&&!stale&&<p>{canMap?'Complete and confirm the track mapping to use this album.':'This release needs a complete tracklist with one release track per local file. Use manual metadata tools for other layouts.'}</p>}
 {editing&&<fieldset disabled={disabled||stale} className="digital-mapping-editor"><legend>Map every file to a release track</legend><p>Choose each track once. File order uses filenames, including their numbers. Check the titles and durations before confirming.</p>
 <button type="button" onClick={()=>update(Object.fromEntries(files.map((t,i)=>[`${t.source}:${t.file}`,slots[i]?.value||''])))}>Assign in file order</button>
 {files.map(t=>{const key=`${t.source}:${t.file}`;const recognized=candidate.mappings.find(m=>m.key===key);return <label key={key}>{t.name} ({duration(t.frames/t.sample_rate*1000)})<small>{recognized?`Audio suggests disc ${recognized.medium}, track ${recognized.position} · AcoustID score ${recognized.score.toFixed(2)}`:'No audio association'}</small><select value={draft[key]||''} onChange={e=>update({...draft,[key]:e.target.value})}><option value="">Choose release track</option>{slots.map(slot=><option key={slot.value} value={slot.value}>Disc {slot.medium}, track {slot.position}: {slot.title} ({duration(slot.duration_ms)})</option>)}</select></label>;})}
 {!complete&&<p role="status">Assign every file to a different release track.</p>}
 <label className="inline-check"><input type="checkbox" checked={confirmed} disabled={!complete} onChange={e=>setConfirmed(e.target.checked)}/>I reviewed all {files.length} file assignments and confirm this mapping.</label>
 </fieldset>}
 <button disabled={disabled||!canApply} onClick={()=>void onChoose(editing?assignments():undefined)}>Use this album and track mapping</button>
 </div></article>;
}
