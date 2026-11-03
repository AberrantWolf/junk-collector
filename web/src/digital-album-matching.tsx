import React,{useEffect,useRef,useState,api,queueDigitalAlbum,requestErrorMessage,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';
type Item=components['schemas']['PresentedItem'];
type Assignment=components['schemas']['DigitalTrackAssignment'];
type Mapping={key:string;medium:number;position:number;recording_id:string;score:number;duration_delta_ms:number|null};
type Candidate={release:{id:string;title:string;artist:string;disambiguation:string|null;date:string|null;country:string|null;media:{position:number|null;format:string|null;track_count:number;tracks:{position:number;title:string;duration_ms?:number|null}[]}[]};mappings:Mapping[];local_tracks:number;release_tracks:number;complete_tracklist:boolean;ambiguous_mapping:boolean;duration_conflicts:number;unmatched:string[]};
type Result={can_expand?:boolean;inventory_revision:number;tracks:{source:string;file:string;name:string;frames:number;sample_rate:number}[];matches:Candidate[];truncated:boolean;artwork:{release_id:string;snapshot_id:string;image_id:string}[];artwork_error:string|null;edition_error?:string|null};
type Progress=components['schemas']['DigitalMatchingProgress'];
type Work={stale:boolean;result_revision:string|null;progress:Progress|null;job:{id:string;state:string;phase:string;error:string|null;retryable:boolean};result:Result|null};
type Status={available:boolean;jobs:Work[]};
const active=new Set(['queued','running','cancel_requested']);
export default function DigitalAlbumMatching({userId,item,metadataRevision,disabled,artworkAvailable,onChoose,onArtwork,onError}:{userId:string;item:Item;metadataRevision:number;disabled:boolean;artworkAvailable:boolean;onChoose:(job:string,release:string,assignments?:Assignment[])=>Promise<void>;onArtwork:(release:string)=>Promise<void>;onError:(error:unknown)=>void}){
 const [status,setStatus]=useState<Status>();const [error,setError]=useState('');const [loadError,setLoadError]=useState('');const [busy,setBusy]=useState(false);const [reload,setReload]=useState(0);
 const lifetime=useRef(new AbortController());const callbacks=useRef({onChoose,onArtwork,onError});callbacks.current={onChoose,onArtwork,onError};
 const owns=(controller:AbortController)=>lifetime.current===controller&&!controller.signal.aborted;
 useEffect(()=>{const controller=new AbortController();lifetime.current=controller;setStatus(undefined);setBusy(false);setError('');setLoadError('');let timer:ReturnType<typeof setTimeout>|undefined;
  let previous:Status|undefined;
  async function load(){try{const known=previous?.jobs.find(work=>work.result)?.result_revision;const next=await api<Status>(`/items/${item.id}/digital-album-matches${known?`?known_result=${encodeURIComponent(known)}`:''}`,{signal:controller.signal});if(controller.signal.aborted)return;next.jobs=next.jobs.map(work=>({...work,result:work.result||(work.result_revision?previous?.jobs.find(old=>old.job.id===work.job.id&&old.result_revision===work.result_revision)?.result:null)||null}));previous=next;setStatus(next);setLoadError('');if(next.jobs.some(w=>active.has(w.job.state)))timer=setTimeout(load,1000);}catch(e){if(!controller.signal.aborted){if(isApiRequestError(e)&&e.code==='unauthenticated')callbacks.current.onError(e);else setLoadError(requestErrorMessage(e));timer=setTimeout(load,3000);}}}
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
 return <section aria-label="Digital album matching" className="digital-album-matching"><h4>Find matching digital albums</h4><p>Identify recordings from the audio when tags are missing. This sends fingerprints and track durations to AcoustID, then finds MusicBrainz releases. Audio files and purchase details stay on this server.</p>
 {status&&!status.available&&<p className="notice">Configure the server’s AcoustID application key and MusicBrainz contact to enable matching.</p>}
 <button disabled={disabled||busy||!!pending||!status?.available} onClick={()=>void start()}>Find matching albums</button>
 {pending&&<div role="status"><p>{matchingProgress(pending)}</p><button disabled={busy||pending.job.state==='cancel_requested'} onClick={()=>void jobAction(pending.job.id,'DELETE')}>Cancel album matching</button></div>}
 {latest?.job.error&&!latest.stale&&<p role="alert">{latest.job.error}</p>}{latest?.job.retryable&&!latest.stale&&!pending&&<button disabled={busy||disabled} onClick={()=>void jobAction(latest.job.id,'POST')}>Retry album matching</button>}{loadError&&<p role="alert">{loadError}</p>}{error&&<p role="alert">{error}</p>}
 {completed?.result&&<>{!pending&&<button disabled={disabled||busy} onClick={()=>void start(false,true)}>Refresh lookups</button>}<div className="digital-match-guidance"><p>Review track mappings before choosing a release.</p><details className="digital-match-info"><summary aria-label="About audio recognition">ⓘ</summary><p>Recognition can be incomplete. These are identity suggestions, not catalog verification. Choosing a release applies the album and every mapped track title; it does not verify the audio.</p></details></div>{inventoryChanged&&<p className="notice">Album inventory or metadata changed. Find matching albums again before applying a release.</p>}{completed.result.truncated&&<p className="notice">Search limited.{!pending&&completed.result.can_expand&&<button disabled={disabled||busy} onClick={()=>void start(true)}>Search more releases</button>}</p>}{completed.result.edition_error&&<p role="alert">Other editions could not be loaded: {completed.result.edition_error}</p>}{completed.result.artwork_error&&<p role="alert">Artwork needs attention: {completed.result.artwork_error}</p>}{!completed.result.matches.length&&<p>No album candidates are available to review. Try a manual MusicBrainz search below.</p>}
 {completed.result.matches.map(candidate=><AlbumCandidate key={`${completed.job.id}:${candidate.release.id}`} candidate={candidate} result={completed.result!} item={item} stale={inventoryChanged} artworkAvailable={artworkAvailable} disabled={disabled||busy||!['completed','queued','running'].includes(completed.job.state)} onChoose={assignments=>callbacks.current.onChoose(completed.job.id,candidate.release.id,assignments)} onArtwork={()=>callbacks.current.onArtwork(candidate.release.id)}/> )}</>}

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
function AlbumCandidate({candidate,result,item,stale,disabled,artworkAvailable,onChoose,onArtwork}:{candidate:Candidate;result:Result;item:Item;stale:boolean;disabled:boolean;artworkAvailable:boolean;onChoose:(assignments?:Assignment[])=>Promise<void>;onArtwork:()=>Promise<void>}){
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
 <strong>{candidate.release.title}</strong><p className="digital-match-artist">{candidate.release.artist}</p>
 <div className="digital-match-edition">{candidate.release.disambiguation&&<span className="digital-match-edition-badge">{candidate.release.disambiguation}</span>}{candidate.release.date&&<span>{candidate.release.date}</span>}{candidate.release.country&&<ReleaseRegion code={candidate.release.country}/>}</div>
 <p>{candidate.mappings.length} of {candidate.local_tracks} files associated by audio recognition · {candidate.release_tracks} release tracks{candidate.ambiguous_mapping?' · ambiguous track assignment':''}{candidate.duration_conflicts?' · conflicting durations':''}</p>
 {candidate.mappings.length<candidate.local_tracks&&<p className="notice">Partial album evidence: the remaining files have no recording association to this release.</p>}
 <details><summary>Review proposed track mapping</summary><p>AcoustID scores describe recording associations, not the probability that the whole album is correct.</p><div className="digital-mapping-table" role="region" aria-label="Proposed track mapping" tabIndex={0}><table><thead><tr><th scope="col">Local file</th><th scope="col">Release track</th><th scope="col">Time gap</th><th scope="col">Score</th></tr></thead><tbody>{files.map(file=>{const key=`${file.source}:${file.file}`;const mapping=candidate.mappings.find(m=>m.key===key);const track=mapping&&slots.find(t=>t.medium===mapping.medium&&t.position===mapping.position);return <tr key={key}><td data-label="Local file">{file.name}</td><td data-label="Release track">{mapping?<><span className="mapping-position">Disc {mapping.medium} · track {mapping.position}</span>{track?.title||'Untitled'}</>:'No recording association'}</td><td data-label="Duration difference">{mapping?.duration_delta_ms!=null?`${(mapping.duration_delta_ms/1000).toFixed(1)} s`:'Unknown'}</td><td data-label="Score">{mapping?.score.toFixed(2)||'—'}</td></tr>;})}</tbody></table></div>{slots.some(slot=>!candidate.mappings.some(m=>m.medium===slot.medium&&m.position===slot.position))&&<div><h5>Release tracks without a local file</h5><ul>{slots.filter(slot=>!candidate.mappings.some(m=>m.medium===slot.medium&&m.position===slot.position)).map(slot=><li key={slot.value}>Disc {slot.medium} · track {slot.position}: {slot.title}</li>)}</ul></div>}</details>
 {!stale&&canMap&&<button disabled={disabled} onClick={()=>{setEditing(v=>!v);setConfirmed(false);}}>{editing?'Close mapping editor':'Edit track mapping'}</button>}
 {!automatic&&!editing&&!stale&&<p>{canMap?'Complete and confirm the track mapping to use this album.':`This release cannot be applied: ${!candidate.complete_tracklist?'MusicBrainz’s release track list is incomplete. ':''}${candidate.release_tracks!==files.length?`It declares ${candidate.release_tracks} release tracks for ${files.length} local files. `:''}${slots.length!==files.length?`${slots.length} release track entries were received. `:''}Review the track counts below or choose another edition.`}</p>}
 {!canMap&&<ul className="digital-track-counts">{candidate.release.media.map((medium,index)=><li key={index}>Disc {medium.position??'?'}: {medium.track_count} declared tracks · {medium.tracks.length} received</li>)}</ul>}
 {editing&&<fieldset disabled={disabled||stale} className="digital-mapping-editor"><legend>Map every file to a release track</legend><p>Choose each track once. File order uses filenames, including their numbers. Check the titles and durations before confirming.</p>
 <button type="button" onClick={()=>update(Object.fromEntries(files.map((t,i)=>[`${t.source}:${t.file}`,slots[i]?.value||''])))}>Assign in file order</button>
 {files.map(t=>{const key=`${t.source}:${t.file}`;const recognized=candidate.mappings.find(m=>m.key===key);return <label key={key}>{t.name} ({duration(t.frames/t.sample_rate*1000)})<small>{recognized?`Audio suggests disc ${recognized.medium}, track ${recognized.position} · AcoustID score ${recognized.score.toFixed(2)}`:'No audio association'}</small><select value={draft[key]||''} onChange={e=>update({...draft,[key]:e.target.value})}><option value="">Choose release track</option>{slots.map(slot=><option key={slot.value} value={slot.value}>Disc {slot.medium}, track {slot.position}: {slot.title} ({duration(slot.duration_ms)})</option>)}</select></label>;})}
 {!complete&&<p role="status">Assign every file to a different release track.</p>}
 <label className="inline-check"><input type="checkbox" checked={confirmed} disabled={!complete} onChange={e=>setConfirmed(e.target.checked)}/>I reviewed all {files.length} file assignments and confirm this mapping.</label>
 </fieldset>}
 <button disabled={disabled||!artworkAvailable} onClick={()=>void onArtwork()}>Choose artwork only</button>
 <button disabled={disabled||!canApply} onClick={()=>void onChoose(editing?assignments():undefined)}>Use this album and track mapping</button>
 </div></article>;
}
