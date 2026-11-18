import React,{useEffect,useRef,useState,api,queueDigitalAlbum,requestErrorMessage,isApiRequestError,retainUnchanged} from '@collector/runtime';
import type {components} from './generated/api';
type Item=components['schemas']['PresentedItem'];
type Assignment=components['schemas']['DigitalTrackAssignment'];
type Mapping={key:string;medium:number;position:number;recording_id:string;score:number;duration_delta_ms:number|null};
type ReleaseTrack={position:number;title:string;duration_ms?:number|null;recording_id?:string|null};
type ReleaseSlot=ReleaseTrack&{medium:number;value:string};
type LocalFile={source:string;file:string;name:string;frames:number;sample_rate:number;disc?:number|null;position?:number|null};
type SubmissionOutcome=components['schemas']['AcoustIdSubmissionOutcome'];
type Candidate={release:{id:string;title:string;artist:string;disambiguation:string|null;date:string|null;country:string|null;media:{position:number|null;format:string|null;track_count:number;tracks:ReleaseTrack[]}[]};mappings:Mapping[];local_tracks:number;release_tracks:number;complete_tracklist:boolean;ambiguous_mapping:boolean;duration_conflicts:number;unmatched:string[]};
type Result={can_expand?:boolean;inventory_revision:number;tracks:LocalFile[];matches:Candidate[];truncated:boolean;artwork:{release_id:string;snapshot_id:string;image_id:string}[];artwork_error:string|null;edition_error?:string|null};
type Progress=components['schemas']['DigitalMatchingProgress'];
type Work={stale:boolean;submission_files?:string[];result_revision:string|null;progress:Progress|null;job:{id:string;state:string;phase:string;error:string|null;retryable:boolean};result:Result|null};
type Status={available:boolean;jobs:Work[]};
const active=new Set(['queued','running','cancel_requested']);
export default function DigitalAlbumMatching({userId,item,metadataRevision,disabled,artworkAvailable,onChoose,onArtwork,onError}:{userId:string;item:Item;metadataRevision:number;disabled:boolean;artworkAvailable:boolean;onChoose:(job:string,release:string,assignments?:Assignment[])=>Promise<void>;onArtwork:(release:string)=>Promise<void>;onError:(error:unknown)=>void}){
 const [status,setStatus]=useState<Status>();const [error,setError]=useState('');const [loadError,setLoadError]=useState('');const [busy,setBusy]=useState(false);const [reload,setReload]=useState(0);
 const lifetime=useRef(new AbortController());const callbacks=useRef({onChoose,onArtwork,onError});callbacks.current={onChoose,onArtwork,onError};
 const owns=(controller:AbortController)=>lifetime.current===controller&&!controller.signal.aborted;
 useEffect(()=>{const controller=new AbortController();lifetime.current=controller;setStatus(undefined);setBusy(false);setError('');setLoadError('');let timer:ReturnType<typeof setTimeout>|undefined;
  let previous:Status|undefined;
  async function load(){try{const known=previous?.jobs.find(work=>work.result)?.result_revision;const next=await api<Status>(`/items/${item.id}/digital-album-matches${known?`?known_result=${encodeURIComponent(known)}`:''}`,{signal:controller.signal});if(controller.signal.aborted)return;next.jobs=next.jobs.map(work=>({...work,result:work.result||(work.result_revision?previous?.jobs.find(old=>old.job.id===work.job.id&&old.result_revision===work.result_revision)?.result:null)||null}));if(retainUnchanged(previous,next)!==previous){previous=next;setStatus(next);}setLoadError('');if(next.jobs.some(w=>active.has(w.job.state)))timer=setTimeout(load,1000);}catch(e){if(!controller.signal.aborted){if(isApiRequestError(e)&&e.code==='unauthenticated')callbacks.current.onError(e);else setLoadError(requestErrorMessage(e));timer=setTimeout(load,3000);}}}
  void load();return()=>{controller.abort();if(timer)clearTimeout(timer);};
 },[userId,item.id,item.revision,metadataRevision,reload]);
 const pending=status?.jobs.find(w=>active.has(w.job.state));const latest=status?.jobs[0];const completed=status?.jobs.find(w=>w.result);const inventoryChanged=!!completed?.result&&(completed.stale!==false||completed.result.inventory_revision!==item.revision);
 async function start(extended=false,refresh=false){
  const controller=lifetime.current;setBusy(true);setError('');
  try{await queueDigitalAlbum(userId,item.id,item.title,item.revision,extended,refresh);if(owns(controller))setReload(v=>v+1);}
  catch(e){if(owns(controller)){if(isApiRequestError(e)&&e.code==='unauthenticated')callbacks.current.onError(e);else if(e instanceof DOMException&&['QuotaExceededError','SecurityError'].includes(e.name))setError(requestErrorMessage(e));}}
  finally{if(owns(controller))setBusy(false);}
 }

 async function jobAction(job:string,method:'POST'|'DELETE'){const controller=lifetime.current;setError('');setBusy(true);try{await api(`/jobs/${job}${method==='POST'?'/retry':''}`,{method,signal:controller.signal});if(owns(controller))setReload(v=>v+1);}catch(e){if(owns(controller))setError(requestErrorMessage(e));}finally{if(owns(controller))setBusy(false);}}
 if(!item.captures.some(c=>c.representations.some(r=>r.format==='original-file-audio-package')))return null;
 return <section aria-label="Digital album matching" className="metadata-source metadata-search digital-album-matching"><h4>Find matching digital albums</h4><p className="muted">Search MusicBrainz releases using AcoustID audio fingerprints.</p>
 {status&&!status.available&&<p className="notice">Configure the server’s AcoustID application key and MusicBrainz contact to enable matching.</p>}
 <button className="primary" disabled={disabled||busy||!!pending||!status?.available} onClick={()=>void start()}>Find matching albums</button>
 {pending&&<div role="status"><p>{matchingProgress(pending)}</p><button disabled={busy||pending.job.state==='cancel_requested'} onClick={()=>void jobAction(pending.job.id,'DELETE')}>Cancel album matching</button></div>}
 {latest?.job.error&&!latest.stale&&<p role="alert">{latest.job.error}</p>}{latest?.job.retryable&&!latest.stale&&!pending&&<button disabled={busy||disabled} onClick={()=>void jobAction(latest.job.id,'POST')}>Retry album matching</button>}{loadError&&<p role="alert">{loadError}</p>}{error&&<p role="alert">{error}</p>}
 {completed?.result&&<>{!pending&&<button disabled={disabled||busy} onClick={()=>void start(false,true)}>Refresh lookups</button>}<div className="digital-match-guidance"><p>Review track mappings before choosing a release.</p><details className="digital-match-info"><summary aria-label="About audio recognition">ⓘ</summary><p>Recognition can be incomplete. These are identity suggestions, not catalog verification. Choosing a release applies the album and every mapped track title; it does not verify the audio.</p></details></div>{inventoryChanged&&<p className="notice">Album inventory or metadata changed. Find matching albums again before applying a release.</p>}{completed.result.truncated&&<p className="notice">Search limited.{!pending&&completed.result.can_expand&&<button disabled={disabled||busy} onClick={()=>void start(true)}>Search more releases</button>}</p>}{completed.result.edition_error&&<p role="alert">Other editions could not be loaded: {completed.result.edition_error}</p>}{completed.result.artwork_error&&<p role="alert">Artwork needs attention: {completed.result.artwork_error}</p>}{!completed.result.matches.length&&<p>No album candidates are available to review. Try a manual MusicBrainz search below.</p>}
 {completed.result.matches.map(candidate=><AlbumCandidate key={`${completed.job.id}:${candidate.release.id}`} candidate={candidate} result={completed.result!} item={item} jobId={completed.job.id} submissionFiles={completed.submission_files||[]} stale={inventoryChanged} artworkAvailable={artworkAvailable} disabled={disabled||busy||!['completed','queued','running'].includes(completed.job.state)} onChoose={assignments=>callbacks.current.onChoose(completed.job.id,candidate.release.id,assignments)} onArtwork={()=>callbacks.current.onArtwork(candidate.release.id)}/> )}</>}

 </section>;
}

function matchingProgress(work:Work){
 if(work.job.state==='cancel_requested')return 'Cancelling…';
 const p=work.progress;if(!p)return work.job.state==='queued'?'Queued':'Starting…';
 if(p.fingerprints<p.tracks||p.lookups<p.tracks)return `Fingerprints ${p.fingerprints}/${p.tracks} · Lookups ${p.lookups}/${p.tracks}`;
 if(p.editions_pending)return 'Matches ready · finding other editions';
 if(p.recordings<p.recordings_total)return `Finding releases · ${p.recordings}/${p.recordings_total}`;
 if(p.releases<p.releases_total)return `Fetching release track lists · ${p.releases}/${p.releases_total}`;
 return p.artwork_pending?'Matches ready · loading covers':'Comparing releases…';
}
function duration(ms:number|null|undefined){return ms==null?'duration unknown':`${Math.floor(ms/60000)}:${String(Math.floor(ms/1000)%60).padStart(2,'0')}`;}
function ReleaseRegion({code}:{code:string}){
 const region=code.toUpperCase();
 let name=region==='XW'?'Worldwide':region;
 if(region!=='XW')try{name=new Intl.DisplayNames(['en'],{type:'region'}).of(region)||region;}catch{}
 const flag=/^[A-Z]{2}$/.test(region)&&name!==region&&!region.startsWith('X')&&region!=='ZZ'?String.fromCodePoint(...[...region].map(c=>127397+c.charCodeAt(0))):'🌐';
 return <span className="digital-match-region"><span aria-hidden="true">{flag}</span> {name}</span>;
}
function fileKey(file:LocalFile){return `${file.source}:${file.file}`;}
function measuredDuration(file:LocalFile){return file.frames/file.sample_rate*1000;}
function normalizedTitle(title:string){
 return title.normalize('NFKD').toLowerCase().replace(/[^\p{L}\p{N}]/gu,'');
}

function tentativeAssignments(files:LocalFile[],slots:ReleaseSlot[],mappings:Mapping[]){
 const matchedKeys=new Set(mappings.map(mapping=>mapping.key));
 const occupied=new Set(mappings.map(mapping=>`${mapping.medium}:${mapping.position}`));
 const available=slots.filter(slot=>!occupied.has(slot.value));
 const proposals=new Map<string,ReleaseSlot>();
 const mediumCount=new Set(slots.map(slot=>slot.medium)).size;
 for(const file of files){
  const key=fileKey(file);
  if(matchedKeys.has(key))continue;
  const parts=file.name.split('/');
  const stem=parts.pop()!.replace(/\.[^.]+$/,'');
  const prefix=stem.match(/^(\d{1,3})(?:[ ._-]+|$)/);
  const position=file.position??(prefix?Number(prefix[1]):null);
  const folder=parts.map(part=>part.match(/^(?:disc|cd)[ ._-]*(\d{1,2})$/i)).find(Boolean);
  const disc=file.disc??(folder?Number(folder[1]):null);
  const title=normalizedTitle(prefix?stem.slice(prefix[0].length):stem);
  const candidates=available.filter(slot=>{
   if(disc!=null&&disc!==slot.medium)return false;
   const gap=slot.duration_ms==null?null:Math.abs(measuredDuration(file)-slot.duration_ms);
   if(gap!=null&&gap>2000)return false;
   const samePosition=position===slot.position&&(disc!=null||mediumCount===1);
   const sameTitle=!!title&&title===normalizedTitle(slot.title);
   return sameTitle||samePosition&&gap!=null;
  });
  if(candidates.length===1)proposals.set(key,candidates[0]);
 }
 const counts=new Map<string,number>();
 for(const slot of proposals.values())counts.set(slot.value,(counts.get(slot.value)||0)+1);
 return Object.fromEntries([...proposals].filter(([,slot])=>counts.get(slot.value)===1).map(([key,slot])=>[key,slot.value]));
}

function AlbumCandidate({candidate,result,item,jobId,submissionFiles,stale,disabled,artworkAvailable,onChoose,onArtwork}:{
 candidate:Candidate;result:Result;item:Item;jobId:string;submissionFiles:string[];stale:boolean;disabled:boolean;
 artworkAvailable:boolean;onChoose:(assignments?:Assignment[])=>Promise<void>;onArtwork:()=>Promise<void>;
}){
 const automatic=candidate.complete_tracklist&&!candidate.ambiguous_mapping&&!candidate.duration_conflicts&&candidate.mappings.length===candidate.local_tracks&&candidate.release_tracks===candidate.local_tracks;
 const [editing,setEditing]=useState(false);
 const [confirmed,setConfirmed]=useState(false);
 const [draft,setDraft]=useState<Record<string,string>>({});
 const [ordered,setOrdered]=useState<Record<string,string>>({});
 const [submitting,setSubmitting]=useState(false);
 const slots=candidate.release.media.flatMap(m=>m.position==null?[]:m.tracks.map(t=>({...t,medium:m.position!,value:`${m.position}:${t.position}`}))).sort((a,b)=>a.medium-b.medium||a.position-b.position);
 const files=[...result.tracks].sort((a,b)=>a.name.localeCompare(b.name,undefined,{numeric:true})||fileKey(a).localeCompare(fileKey(b)));
 const recognized=new Map(candidate.mappings.map(mapping=>[mapping.key,mapping]));
 const suggestions=tentativeAssignments(files,slots,candidate.mappings);
 const suggestedCount=Object.keys(suggestions).length;
 const proposedSlots=new Set([...Object.values(suggestions),...candidate.mappings.map(mapping=>`${mapping.medium}:${mapping.position}`)]);
 const unassigned=slots.filter(slot=>!proposedSlots.has(slot.value));
 const selected=(key:string)=>draft[key]??suggestions[key]??(recognized.has(key)?`${recognized.get(key)!.medium}:${recognized.get(key)!.position}`:'');
 const slotsByValue=new Map(slots.map(slot=>[slot.value,slot]));
 const canMap=candidate.complete_tracklist&&slots.length===files.length&&candidate.release_tracks===files.length;
 const values=files.map(t=>selected(fileKey(t)));
 const complete=canMap&&values.every(v=>slots.some(s=>s.value===v))&&new Set(values).size===files.length;
 const canApply=!stale&&(editing?complete&&confirmed:automatic);
 const cover=result.artwork.find(a=>a.release_id===candidate.release.id);
 function update(next:Record<string,string>){setDraft(next);setConfirmed(false);}
 const assignments=()=>files.map(file=>{
  const key=fileKey(file);
  const slot=slotsByValue.get(selected(key))!;
   return {key,medium:slot.medium,position:slot.position};
 }).sort((a,b)=>a.key.localeCompare(b.key));
 const submissionCount=submissionFiles.filter(key=>slotsByValue.get(selected(key))?.recording_id).length;
 function assignInFileOrder(){
  const inOrder=Object.fromEntries(files.map((file,index)=>[fileKey(file),slots[index]?.value||'']));
  setOrdered(inOrder);
  update(inOrder);
 }
 return <article className="candidate"><div className="digital-match-cover">{cover?<img loading="lazy" src={`/api/v1/items/${item.id}/metadata/artwork/${cover.snapshot_id}/${encodeURIComponent(cover.image_id)}/thumbnail`} alt={`Cover of ${candidate.release.title}`}/>:<span role="img" aria-label="No cover preview">♫</span>}</div><div>
 <strong>{candidate.release.title}</strong><p className="digital-match-artist">{candidate.release.artist}</p>
 <div className="digital-match-edition">{candidate.release.disambiguation&&<span className="digital-match-edition-badge">{candidate.release.disambiguation}</span>}{candidate.release.date&&<span>{candidate.release.date}</span>}{candidate.release.country&&<ReleaseRegion code={candidate.release.country}/>}</div>
 <p>{candidate.mappings.length} of {candidate.local_tracks} files associated by audio recognition · {candidate.release_tracks} release tracks{candidate.ambiguous_mapping?' · ambiguous track assignment':''}{candidate.duration_conflicts?' · conflicting durations':''}</p>
 {candidate.mappings.length<candidate.local_tracks&&<p className="notice">Partial album evidence: the remaining files have no recording association to this release.</p>}
 <details>
  <summary>Review proposed track mapping</summary>
  <p>AcoustID scores describe recording associations, not the probability that the whole album is correct.</p>
  <div className="digital-mapping-table" role="region" aria-label="Proposed track mapping" tabIndex={0}>
   <table>
    <thead><tr><th scope="col">Local file</th><th scope="col">Release track</th><th scope="col">Time gap</th><th scope="col">Match</th></tr></thead>
    <tbody>{files.map(file=>{
     const key=fileKey(file);
     const mapping=recognized.get(key);
     const slot=slotsByValue.get(mapping?`${mapping.medium}:${mapping.position}`:suggestions[key]);
     const gap=slot?.duration_ms==null?null:Math.abs(measuredDuration(file)-slot.duration_ms);
     return <tr key={key}>
      <td data-label="Local file">{file.name}</td>
      <td data-label="Release track">{slot?<><span className="mapping-position">Disc {slot.medium} · track {slot.position}</span>{slot.title}</>:'No proposed assignment'}</td>
      <td data-label="Duration difference">{gap==null?'Unknown':`${(gap/1000).toFixed(1)} s`}</td>
      <td data-label="Match">{mapping?mapping.score.toFixed(2):slot?<span className="digital-review-tentative">Tentative</span>:'—'}</td>
     </tr>;
    })}</tbody>
   </table>
  </div>
  {unassigned.length>0&&<div><h5>Release tracks without a proposed assignment</h5><ul>{unassigned.map(slot=><li key={slot.value}>Disc {slot.medium} · track {slot.position}: {slot.title}</li>)}</ul></div>}
 </details>
 {!stale&&canMap&&<button disabled={disabled||submitting} onClick={()=>{setEditing(v=>!v);setConfirmed(false);}}>{editing?'Close mapping editor':'Edit track mapping'}</button>}
 {!automatic&&!editing&&!stale&&<p>{canMap?'Complete and confirm the track mapping to use this album.':`This release cannot be applied: ${!candidate.complete_tracklist?'MusicBrainz’s release track list is incomplete. ':''}${candidate.release_tracks!==files.length?`It declares ${candidate.release_tracks} release tracks for ${files.length} local files. `:''}${slots.length!==files.length?`${slots.length} release track entries were received. `:''}Review the track counts below or choose another edition.`}</p>}
 {!canMap&&<ul className="digital-track-counts">{candidate.release.media.map((medium,index)=><li key={index}>Disc {medium.position??'?'}: {medium.track_count} declared tracks · {medium.tracks.length} received</li>)}</ul>}
 {editing&&<fieldset disabled={disabled||stale||submitting} className="digital-mapping-editor">
  <legend>Map every file to a release track</legend>
  <p>Choose each track once. Check the titles and durations before confirming.</p>
  {suggestedCount>0&&<p className="notice">{suggestedCount} tentative {suggestedCount===1?'assignment uses':'assignments use'} filenames and track details. Check each suggestion.</p>}
  <button type="button" onClick={assignInFileOrder}>Assign in file order</button>
  <div className="digital-review-heading" aria-hidden="true"><b>Local file</b><b>Release track</b></div>
  {files.map(file=>{
   const key=fileKey(file);
   const audio=recognized.get(key);
   const value=selected(key);
   const slot=slotsByValue.get(value);
   const audioMatches=audio&&value===`${audio.medium}:${audio.position}`;
   const tentative=!!value&&!audioMatches&&(value===suggestions[key]||value===ordered[key]);
   return <div className="digital-review-pair" key={key}>
    <div className="digital-review-file">
     <strong>{file.name}</strong>
     <small>{duration(measuredDuration(file))}</small>
     {audio&&!audioMatches&&<small>AcoustID suggests disc {audio.medium}, track {audio.position} (score {audio.score.toFixed(3)})</small>}
    </div>
    <div className="digital-review-track">
     {slot&&<><strong>{slot.title}</strong><small>Disc {slot.medium} · track {slot.position} · {duration(slot.duration_ms)}</small></>}
     <select aria-label={`Release track for ${file.name}`} value={value} onChange={event=>{
      const nextOrdered={...ordered};delete nextOrdered[key];setOrdered(nextOrdered);
      update({...draft,[key]:event.target.value});
     }}>
      <option value="">Choose release track</option>
      {slots.map(option=><option key={option.value} value={option.value}>Disc {option.medium}, track {option.position}: {option.title} ({duration(option.duration_ms)})</option>)}
     </select>
     {slot&&<div className="digital-review-evidence">
      {audioMatches?<span>{audio.score>=.95?'Strong AcoustID match':'AcoustID match'} ({audio.score.toFixed(3)})</span>
       :tentative?<span className="digital-review-tentative">Tentative · {value===suggestions[key]?'file and track details':'file order'}</span>
       :<span>Manual assignment</span>}
      {slot.duration_ms!=null&&<small>{(Math.abs(measuredDuration(file)-slot.duration_ms)/1000).toFixed(1)} s duration difference</small>}
     </div>}
     {!slot&&<small>No release track selected</small>}
    </div>
   </div>;
  })}
  {!complete&&<p role="status">Assign every file to a different release track.</p>}
  <label className="inline-check"><input type="checkbox" checked={confirmed} disabled={!complete} onChange={event=>{
   if(event.target.checked)setDraft(Object.fromEntries(files.map(file=>[fileKey(file),selected(fileKey(file))])));
   setConfirmed(event.target.checked);
  }}/>I reviewed all {files.length} file assignments and confirm this mapping.</label>
  {submissionCount>0&&complete&&<AcoustIdSubmission
   itemId={item.id} jobId={jobId} releaseId={candidate.release.id} assignments={assignments()}
   count={submissionCount} confirmed={confirmed} onBusy={setSubmitting}/>}
 </fieldset>}
 <button disabled={disabled||submitting||!artworkAvailable} onClick={()=>void onArtwork()}>Choose artwork only</button>
 <button disabled={disabled||submitting||!canApply} onClick={()=>void onChoose(editing?assignments():undefined)}>Use this album and track mapping</button>
 </div></article>;
}

function AcoustIdSubmission({itemId,jobId,releaseId,assignments,count,confirmed,onBusy}:{
 itemId:string;jobId:string;releaseId:string;assignments:Assignment[];count:number;confirmed:boolean;onBusy:(busy:boolean)=>void;
}){
 const [userKey,setUserKey]=useState('');
 const [outcome,setOutcome]=useState<SubmissionOutcome>();
 const [error,setError]=useState('');
 const request=JSON.stringify({job_id:jobId,release_id:releaseId,track_assignments:assignments});
 const lifetime=useRef(new AbortController());
 useEffect(()=>{
  const controller=new AbortController();
  lifetime.current=controller;
  setOutcome(undefined);
  setError('');
  return()=>controller.abort();
 },[request]);
 async function submit(){
  const {signal}=lifetime.current;
  setError('');
  onBusy(true);
  try{
   const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(request));
   const key=Array.from(new Uint8Array(digest),byte=>byte.toString(16).padStart(2,'0')).join('');
   const response=await api<SubmissionOutcome>(`/items/${itemId}/acoustid-submissions`,{
    method:'POST',
    headers:{'Idempotency-Key':key},
    body:JSON.stringify({...JSON.parse(request),user_key:userKey.trim()}),
    signal:AbortSignal.any([signal,AbortSignal.timeout(35000)]),
   });
   if(!signal.aborted){setOutcome(response);setUserKey('');}
  }catch(e){
   if(!signal.aborted)setError(e instanceof DOMException&&e.name==='TimeoutError'
    ?'The request timed out. Submit the same mapping again to check its receipt.'
    :requestErrorMessage(e));
  }finally{
   onBusy(false);
  }
 }
 return <details className="acoustid-contribution">
  <summary>Contribute missing fingerprints to AcoustID</summary>
  <p>Send {count} {count===1?'fingerprint':'fingerprints'}, file durations, and reviewed MusicBrainz recording IDs to AcoustID.</p>
  {!outcome&&<>
   <label>AcoustID user API key<input type="password" autoComplete="off" maxLength={128} value={userKey} onChange={event=>setUserKey(event.target.value)}/></label>
   <p><a href="https://acoustid.org/api-key" target="_blank" rel="noreferrer">Get your AcoustID user key</a></p>
   <button type="button" disabled={!confirmed||!userKey.trim()} onClick={()=>void submit()}>Submit {count} {count===1?'fingerprint':'fingerprints'} to AcoustID</button>
  </>}
  {outcome?.status==='accepted'&&<p role="status">AcoustID accepted the contribution for processing. Submission IDs {outcome.submission_ids.join(', ')}. Use Refresh lookups after AcoustID processes it.</p>}
  {outcome?.status==='unknown'&&<p role="alert">AcoustID did not confirm receipt. Check AcoustID before attempting another contribution.</p>}
  {error&&<p role="alert">{error}</p>}
 </details>;
}
