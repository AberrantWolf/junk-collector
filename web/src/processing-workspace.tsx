import {presentationLabel} from './presentation-labels';
import React,{useEffect,useState,useRef,api} from '@collector/runtime';
import type {components} from './generated/api';


type Item=components['schemas']['PresentedItem'];
type Capture=components['schemas']['PresentedCapture'];
type Representation=components['schemas']['Representation'];
type Capability=components['schemas']['CapabilityAffordance'];
type SnapshotPage=components['schemas']['Page_CatalogSnapshotStatus'];
type SnapshotStatus=components['schemas']['CatalogSnapshotStatus'];
type SummaryPage=components['schemas']['Page_ProcessingSummary'];
type Summary=components['schemas']['ProcessingSummary'];
type ScanPlan=components['schemas']['ScanPlan'];
type JobView=components['schemas']['JobView'];
type Props={item:Item;busy:boolean;setBusy:(value:boolean)=>void;onError:(error:unknown)=>void;focusSourceId?:string;focusOperation?:'structure'|'accuraterip'};

const terminal=new Set<JobView['state']>(['completed','failed','cancelled']);
const operations:Record<string,string>={
 'org.junk.games.hash':'Hash game data',
 'org.junk.games.inspect-ines':'Inspect iNES structure',
 'org.junk.games.hash-tracks':'Hash raw 2352-byte tracks',
 'org.junk.games.catalog-verification':'Compare with a DAT snapshot',
 'org.junk.music.cd-structure':'Inspect CD structure',
 'org.junk.music.accuraterip-verification':'Verify audio with AccurateRip',
};
const operationLabels:Record<string,string>={...operations,'org.junk.music.import-analysis':'CD import analysis','org.junk.games.import-analysis':'Game import analysis'};

function object(value:unknown):Record<string,unknown>{return value!==null&&typeof value==='object'&&!Array.isArray(value)?value as Record<string,unknown>:{};}
function snapshotLabel(status:SnapshotStatus){const value=object(status.snapshot.parsed_observation);return `${String(value.catalog||'DAT catalog')} ${String(value.version||'unversioned')} · ${String(value.system||'unknown system')}${status.active?' · active':''}`;}
function musicSnapshotLabel(status:SnapshotStatus){const value=object(status.snapshot.parsed_observation);const query=object(value.query);return `AccurateRip ${String(value.outcome||'response')} · ${String(query.track_count||'?')} tracks · ${new Date(status.snapshot.created_at).toLocaleString()}`;}
function resultLabel(summary:{result:unknown}):string{
 const result=object(summary.result);const catalog=object(result.catalog);const kind=String(catalog.match||'');
 const candidates=kind==='unique_complete_match'?[object(catalog.candidate)]:Array.isArray(catalog.candidates)?catalog.candidates.map(object):[];
 const matched=candidates.flatMap(candidate=>Array.isArray(candidate.matched_units)?candidate.matched_units.map(object):[]);
 const crcOnly=matched.length>0&&matched.every(unit=>object(unit.digest).algorithm==='crc32');
 const checksumNote=crcOnly?' (CRC32 only; not cryptographic identity)':'';
 if(kind==='unique_complete_match')return `Exact catalog match${checksumNote}`;
 if(kind==='ambiguous_complete_matches')return `Exact but ambiguous catalog matches${checksumNote}`;
 if(kind==='partial_identification')return `Partial identification only${checksumNote}`;
 if(kind==='no_match')return 'No catalog match';
 if(kind==='inconclusive')return `Inconclusive: ${String(catalog.reason||'insufficient evidence')}`;
 if(result.result==='hashes')return 'File hashes recorded';
 if(result.result==='track_hashes')return 'Track hashes recorded';
 if(result.result==='structure')return `${String(result.format||'Game')} structure inspected`;
 if(result.result==='layout')return `CD structure recorded · ${Array.isArray(result.tracks)?result.tracks.length:0} tracks`;
 if(result.result==='audio_verification'){const verification=object(result.verification);return `AccurateRip: ${presentationLabel(String(verification.status||'unknown'))}`;}
 if(Array.isArray(result.observations))return result.observations.map(value=>resultLabel({...summary,result:object(value)})).join(' · ');
 return 'Recorded check';
}

function ResultDetails({summary}:{summary:{result:unknown}}){
 const result=object(summary.result);
 if(Array.isArray(result.observations))return <>{result.observations.map((value,index)=><ResultDetails key={index} summary={{...summary,result:object(value)}}/>)}</>;
 if(result.result==='layout'){
  const tracks=Array.isArray(result.tracks)?result.tracks.map(object):[];
  return <small>{tracks.map(track=>`Track ${track.position}: INDEX 01 ${track.index_01} sectors${track.pcm_length==null?' · structural only':` · ${track.pcm_length} PCM frames`}`).join(' · ')}</small>;
 }
 if(result.result==='audio_verification'){
  const verification=object(result.verification);
  const tracks=Array.isArray(verification.tracks)?verification.tracks.map(object):[];
  return <>
   {verification.selected_offset!=null&&<small>Selected offset {String(verification.selected_offset)} stereo sample frames</small>}
   {tracks.map(track=>{
    const coverage=object(track.coverage);
    const checksums=Array.isArray(track.checksums)?track.checksums.map(object):[];
    const offsets=Array.isArray(coverage.v2_evaluated_offsets)?coverage.v2_evaluated_offsets:[];
    return <small key={String(track.position)}>
     Track {String(track.position)}: {presentationLabel(String(track.status))}, computed at {String(track.computed_offset)} stereo sample frames.
     {' '}{checksums.map(checksum=>`${String(checksum.version)}: ${String(object(checksum.checksum).hex)} (${checksum.confidence==null?'no database match':`confidence ${checksum.confidence}`})`).join(' · ')}
     {' '}ARv1 interval {String(coverage.v1_evaluated_offset_start)}…{String(coverage.v1_evaluated_offset_end)}; ARv2 evaluated offsets: {offsets.length<=16?offsets.join(', '):`${offsets.length} offsets (${offsets[0]}…${offsets[offsets.length-1]}; expand for exact list)`}.
     {offsets.length>16&&<details><summary>Exact evaluated offsets</summary>{offsets.join(', ')}</details>}
     {' '}Unavailable physical edge frames: leading {String(coverage.unavailable_leading_frames)}, trailing {String(coverage.unavailable_trailing_frames)}.
     {Array.isArray(track.limitations)&&track.limitations.length>0&&` ${track.limitations.join(' · ')}`}
    </small>;
   })}
   {Array.isArray(verification.limitations)&&verification.limitations.length>0&&<small>{verification.limitations.join(' · ')}</small>}
  </>;
 }
 return null;
}

function RepresentationOperations({item,capture,representation,capabilities,snapshots,onError,onSnapshotsChanged,focusOperation}:{item:Item;capture:Capture;representation:Representation;capabilities:Capability[];snapshots:SnapshotStatus[];busy:boolean;setBusy:(value:boolean)=>void;onError:(error:unknown)=>void;onSnapshotsChanged:()=>Promise<void>;focusOperation?:'structure'|'accuraterip'}){
 const lifecycle=useRef(new AbortController());const historyGeneration=useRef(0);
 useEffect(()=>{lifecycle.current=new AbortController();return()=>lifecycle.current.abort();},[]);
 const preferenceKey=`collector.scan.${representation.id}`;
 const [preferences]=useState<Record<string,unknown>>(()=>{try{return object(JSON.parse(sessionStorage.getItem(preferenceKey)||'{}'));}catch{return {};}});
 const available=capabilities.filter(value=>operations[value.id]);
 const [operation,setOperation]=useState(focusOperation==='accuraterip'?'org.junk.music.accuraterip-verification':focusOperation==='structure'?'org.junk.music.cd-structure':typeof preferences.operation==='string'&&available.some(v=>v.id===preferences.operation)?preferences.operation:available.find(value=>value.id==='org.junk.games.hash'||value.id==='org.junk.music.cd-structure')?.id||available.find(value=>value.availability.state==='available')?.id||'');
 const [fileIndex,setFileIndex]=useState<number|null>(typeof preferences.fileIndex==='number'?preferences.fileIndex:null);const [maximumOffset,setMaximumOffset]=useState(typeof preferences.maximumOffset==='number'?preferences.maximumOffset:2939);const [profile,setProfile]=useState(typeof preferences.profile==='string'?preferences.profile:'raw');const [copierHeader,setCopierHeader]=useState(preferences.copierHeader===true);const [catalogInput,setCatalogInput]=useState(typeof preferences.catalogInput==='string'?preferences.catalogInput:'raw_files');const [system,setSystem]=useState(()=>String(preferences.system??object(item.domain.payload).platform??''));const [snapshotId,setSnapshotId]=useState(()=>typeof preferences.snapshotId==='string'?preferences.snapshotId:snapshots.find(value=>value.active)?.snapshot.id||snapshots[0]?.snapshot.id||'');const [summaries,setSummaries]=useState<Summary[]>([]);const [cursor,setCursor]=useState<string|null>(null);
 async function load(after?:string,signal=lifecycle.current.signal){const generation=++historyGeneration.current;const page=await api<SummaryPage>(`/captures/${capture.id}/operations?representation_id=${representation.id}&limit=100${after?`&cursor=${encodeURIComponent(after)}`:''}`,{signal});if(signal.aborted||generation!==historyGeneration.current)return;setSummaries(current=>after?[...current,...page.items]:page.items);setCursor(page.next_cursor??null);}
 useEffect(()=>{load().catch(error=>{if(!lifecycle.current.signal.aborted)onError(error);});},[capture.id,representation.id]);
 useEffect(()=>{if(!snapshots.length)return;setSnapshotId(current=>current&&snapshots.some(value=>value.snapshot.id===current)?current:snapshots.find(value=>value.active)?.snapshot.id||snapshots[0]?.snapshot.id||'');},[snapshots]);
 useEffect(()=>{try{sessionStorage.setItem(preferenceKey,JSON.stringify({operation,fileIndex,maximumOffset,profile,copierHeader,catalogInput,system,snapshotId}));}catch{/* In-memory choices still work when browser storage is unavailable. */}},[operation,fileIndex,maximumOffset,profile,copierHeader,catalogInput,system,snapshotId]);
 function command(){
  let selection:Record<string,unknown>;let frozenSnapshot:string|null=null;
  if(operation==='org.junk.games.catalog-verification'){
   const analysis=catalogInput==='raw_files'?{input:'raw_files'}:catalogInput==='cue_raw2352'?{input:'cue_raw2352',...(fileIndex===null?{}:{cue_file_index:fileIndex})}:{input:'file',...(fileIndex===null?{}:{file_index:fileIndex}),profile,...(['snes_payload','pce_payload'].includes(profile)?{copier_header:copierHeader}:{})};
   selection={snapshot_id:snapshotId,system,...analysis};frozenSnapshot=snapshotId||null;
  }
  else if(operation==='org.junk.music.cd-structure')selection={...(fileIndex===null?{}:{anchor_file_index:fileIndex})};
  else if(operation==='org.junk.music.accuraterip-verification'){selection={...(fileIndex===null?{}:{anchor_file_index:fileIndex}),snapshot_id:snapshotId,maximum_offset:maximumOffset};frozenSnapshot=snapshotId||null;}
  else if(operation==='org.junk.games.hash-tracks')selection={...(fileIndex===null?{}:{cue_file_index:fileIndex})};
  else if(operation==='org.junk.games.inspect-ines')selection={...(fileIndex===null?{}:{file_index:fileIndex})};
  else selection={...(fileIndex===null?{}:{file_index:fileIndex}),profile,...(['snes_payload','pce_payload'].includes(profile)?{copier_header:copierHeader}:{})};
  return {operation,operation_version:1,representation_id:representation.id,snapshot_id:frozenSnapshot,selection};
 }
 const [planned,setPlan]=useState<{request:string;value:ScanPlan}|null>(null);const [planError,setPlanError]=useState('');const [submitting,setSubmitting]=useState(false);const [acquiring,setAcquiring]=useState(false);const [revision,setRevision]=useState(0);const historyRef=useRef<HTMLDivElement>(null);
 const request=JSON.stringify(command());const plan=planned?.request===request?planned.value:null;const busy=submitting;
 useEffect(()=>{
  const controller=new AbortController();let timer:ReturnType<typeof setTimeout>|undefined;
  setPlan(null);setPlanError('');
  async function refresh(){try{const next=await api<ScanPlan>(`/captures/${capture.id}/operations/plan`,{method:'POST',body:request,signal:controller.signal});if(controller.signal.aborted)return;setPlan({request,value:next});if(next.state==='running'||next.state==='cancelling'){timer=setTimeout(()=>void refresh(),1200);}else{await load(undefined,controller.signal);}}catch(error){if(!controller.signal.aborted)setPlanError(error instanceof Error?error.message:String(error));}}
  void refresh();return()=>{controller.abort();if(timer)clearTimeout(timer);};
 },[request,revision]);
 async function run(){if(!plan||submitting||['running','cancelling','blocked'].includes(plan.state))return;setSubmitting(true);try{await api<{job_id:string}>(`/captures/${capture.id}/operations`,{method:'POST',body:JSON.stringify(plan.command)});setRevision(value=>value+1);window.dispatchEvent(new Event('collector-jobs-updated'));}catch(error){onError(error);}finally{setSubmitting(false);}}
 async function acquire(){const structure=summaries.find(summary=>summary.current&&object(summary.result).result==='layout');if(!structure)return;setAcquiring(true);try{const signal=lifecycle.current.signal;const accepted=await api<{job_id:string}>(`/captures/${capture.id}/accuraterip-snapshots`,{signal,method:'POST',body:JSON.stringify({representation_id:representation.id,structure_observation_id:structure.observation_id})});for(;;){signal.throwIfAborted();const job=await api<JobView>(`/jobs/${accepted.job_id}`,{signal});if(terminal.has(job.state)){if(job.state!=='completed')throw new Error(job.error||job.limitation||`AccurateRip request ${job.state}. Preservation and structural evidence remain available.`);break;}await new Promise(resolve=>setTimeout(resolve,1200));}await onSnapshotsChanged();window.dispatchEvent(new Event('collector-jobs-updated'));}catch(error){if(!lifecycle.current.signal.aborted)onError(error);}finally{if(!lifecycle.current.signal.aborted)setAcquiring(false);}}
 const shown=plan?.result&&!summaries.some(value=>value.observation_id===plan.result?.observation_id)?[plan.result,...summaries]:summaries;
 const selected=capabilities.find(value=>value.id===operation);const unavailable=selected?.availability.state==='unavailable'?selected.availability.limitation:'';
 const historyOnly=representation.role!=='preservation_master'&&(!plan||!plan.files.length);
 if(historyOnly&&!shown.length&&!planError)return null;
 return <div className="import-docket"><header><div><span className="docket-phase">{presentationLabel(representation.role)}</span><strong className="docket-title">{representation.files.length===1?representation.files[0].name:representation.format||'Original package'} · {representation.files.length} {representation.files.length===1?'file':'files'}</strong></div></header>
  {planError&&<p className="notice" role="alert">{planError} <button onClick={()=>setRevision(value=>value+1)}>Retry check status</button></p>}
  {!historyOnly&&<><div className="form-grid"><label>Check<select value={operation} disabled={busy} onChange={event=>{setOperation(event.target.value);setFileIndex(null);}}>{available.map(value=><option key={value.id} value={value.id}>{operations[value.id]}{value.availability.state==='unavailable'?' (unavailable)':''}</option>)}</select></label>{!(operation==='org.junk.games.catalog-verification'&&catalogInput==='raw_files')&&<label>Source file<select value={fileIndex??''} disabled={busy} onChange={event=>setFileIndex(event.target.value===''?null:Number(event.target.value))}><option value="">{plan?.files.length===1?`Automatic: ${plan.files[0].label}`:'Choose a file'}</option>{plan?.files.map(file=><option key={file.file_index} value={file.file_index}>{file.label}</option>)}</select></label>}</div>
  {operation==='org.junk.games.hash'&&<><label>File format for checksums<select value={profile} disabled={busy} onChange={event=>setProfile(event.target.value)}><option value="raw">Raw file bytes</option><option value="ines_payload">iNES payload</option><option value="n64_big_endian">Nintendo 64 big-endian</option><option value="snes_payload">SNES payload</option><option value="pce_payload">PC Engine payload</option></select></label>{['snes_payload','pce_payload'].includes(profile)&&<label><input type="checkbox" checked={copierHeader} disabled={busy} onChange={event=>setCopierHeader(event.target.checked)}/> Source includes an explicit 512-byte copier header</label>}</>}
  {operation==='org.junk.games.catalog-verification'&&<><div className="form-grid"><label>DAT system<select value={system} disabled={busy} onChange={event=>setSystem(event.target.value)}><option value="">Choose a detected catalog system</option>{[...new Set(snapshots.map(value=>String(object(value.snapshot.parsed_observation).system||'')).filter(Boolean))].map(value=><option key={value} value={value}>{value}</option>)}</select></label><label>Catalog version<select value={snapshotId} disabled={busy} onChange={event=>setSnapshotId(event.target.value)}>{snapshots.map(value=><option key={value.snapshot.id} value={value.snapshot.id}>{snapshotLabel(value)}</option>)}</select></label></div><div className="form-grid"><label>Files to compare<select value={catalogInput} disabled={busy} onChange={event=>setCatalogInput(event.target.value)}><option value="raw_files">All ordered raw files</option><option value="file">One normalized file profile</option><option value="cue_raw2352">All ordered raw-2352 CUE tracks</option></select></label>{catalogInput==='file'&&<label>File format for checksums<select value={profile} disabled={busy} onChange={event=>setProfile(event.target.value)}><option value="raw">Raw file bytes</option><option value="ines_payload">iNES payload</option><option value="n64_big_endian">Nintendo 64 big-endian</option><option value="snes_payload">SNES payload</option><option value="pce_payload">PC Engine payload</option></select></label>}</div>{catalogInput==='file'&&['snes_payload','pce_payload'].includes(profile)&&<label><input type="checkbox" checked={copierHeader} disabled={busy} onChange={event=>setCopierHeader(event.target.checked)}/> Source includes an explicit 512-byte copier header</label>}</>}
  {operation==='org.junk.games.catalog-verification'&&snapshots.length===0&&<p className="notice">No DAT snapshot is available. Hashing and structural inspection remain usable.</p>}
  {operation==='org.junk.music.accuraterip-verification'&&<><div className="form-grid"><label>Saved AccurateRip response<select value={snapshotId} disabled={busy} onChange={event=>setSnapshotId(event.target.value)}>{snapshots.map(value=><option key={value.snapshot.id} value={value.snapshot.id}>{musicSnapshotLabel(value)}</option>)}</select></label><label>Maximum offset (stereo sample frames)<input type="number" min="0" max="2939" value={maximumOffset} disabled={busy} onChange={event=>setMaximumOffset(Number(event.target.value))}/></label></div>{snapshots.length===0&&<p className="notice">Fetch a response after recording a supported CD structure. A missing response does not mean the audio failed the check.</p>}</>}
  {unavailable&&<p className="notice">{unavailable}</p>}
  {plan?.blockers.map(value=><p className="notice" key={value}>{value}</p>)}
  {plan?.state==='failed'&&<p role="alert">{plan.job?.error||'The last check failed. You can retry.'}</p>}
  {plan?.state==='changed'&&<p className="notice">No result is recorded for these exact inputs. The previous result remains available.</p>}
  {plan?.job&&['running','cancelling'].includes(plan.state)&&<p role="status">{presentationLabel(plan.job.phase)} · {plan.job.completed_units} of {plan.job.total_units} units. Work continues when you close this panel.</p>}
  <div className="button-row">
   {plan?.result&&<button onClick={()=>{const target=document.getElementById(`processing-result-${plan.result!.observation_id}`)||historyRef.current;target?.scrollIntoView({behavior:'smooth',block:'nearest'});target?.focus({preventScroll:true});}}>View result</button>}
   {plan?.state==='running'||plan?.state==='cancelling'?<button disabled>{plan.state==='cancelling'?'Cancelling…':'Check in progress…'}</button>:plan?.result?<details><summary>More actions</summary><button disabled={busy||!plan||plan.state==='blocked'} onClick={()=>void run()}>{plan.state==='changed'?'Check changed inputs':'Check again'}</button></details>:<button disabled={busy||!plan||plan.state==='blocked'||!!unavailable} onClick={()=>void run()}>{submitting?'Starting…':plan?.state==='failed'?'Retry check':'Run check'}</button>}
   {plan?.state==='running'&&plan.job?.cancellable&&<button disabled={busy} onClick={()=>{setSubmitting(true);void api(`/jobs/${plan.job!.id}/cancel`,{method:'POST'}).then(()=>{setRevision(value=>value+1);window.dispatchEvent(new Event('collector-jobs-updated'));}).catch(onError).finally(()=>setSubmitting(false));}}>Cancel check</button>}
   {item.domain.module==='org.junk.music'&&summaries.some(summary=>summary.current&&object(summary.result).result==='layout')&&<button disabled={acquiring} onClick={()=>void acquire()}>{acquiring?'Fetching AccurateRip response…':'Fetch AccurateRip response'}</button>}
  </div>
  </>}
  {shown.length>0&&<div ref={historyRef}><h4>Recorded checks</h4>{shown.map(summary=>{const snapshot=snapshots.find(value=>value.snapshot.id===summary.snapshot_id);return <div className="stored-file" id={`processing-result-${summary.observation_id}`} tabIndex={-1} key={summary.observation_id}><span><strong>{resultLabel(summary)}</strong><small>Checked {summary.occurrences} {summary.occurrences===1?'time':'times'} · Last checked {new Date(summary.last_checked_at).toLocaleString()}<br/>{summary.current?'Current recorded source':'Historical recorded source'} · {operationLabels[summary.operation]||'File check'}<br/>{snapshot?(item.domain.module==='org.junk.music'?musicSnapshotLabel(snapshot):snapshotLabel(snapshot)):summary.snapshot_id?'Saved reference data':'No reference data'}{summary.limitations.length?` · ${summary.limitations.join(' · ')}`:''}</small><details className="processing-raw"><summary>Technical details</summary><p>Check ID: {summary.observation_id}</p><p>Operation: {summary.operation}</p>{summary.snapshot_id&&<p>Reference ID: {summary.snapshot_id}</p>}<ResultDetails summary={summary}/></details></span></div>;})}{cursor&&<button disabled={busy} onClick={()=>void load(cursor).catch(onError)}>Load older checks</button>}</div>}
 </div>;
}

function ProcessingAnalysisWorkspace({item,busy,setBusy,onError,focusSourceId,focusOperation}:Props){
 const snapshotLifecycle=useRef(new AbortController());
 const [snapshots,setSnapshots]=useState<SnapshotStatus[]>([]);const capabilities=item.presentation.capabilities.filter(value=>value.id.startsWith(`${item.domain.module}.`));const music=item.domain.module==='org.junk.music';
 async function loadSnapshots(){const signal=snapshotLifecycle.current.signal;const found:SnapshotStatus[]=[];let cursor:string|null=null;do{const page:SnapshotPage=await api<SnapshotPage>(`/collections/${item.collection_id}/catalog-snapshots?limit=100${cursor?`&cursor=${encodeURIComponent(cursor)}`:''}`,{signal});found.push(...page.items.filter(value=>music?value.snapshot.provider==='org.junk.providers.accuraterip':value.snapshot.provider==='org.junk.providers.local-dat'));cursor=page.next_cursor??null;}while(cursor);if(!signal.aborted)setSnapshots(found);}
 useEffect(()=>{snapshotLifecycle.current=new AbortController();const controller=snapshotLifecycle.current;setSnapshots([]);if(['org.junk.games','org.junk.music'].includes(item.domain.module)&&item.captures.length>0)loadSnapshots().catch(error=>{if(!controller.signal.aborted)onError(error);});return()=>controller.abort();},[item.collection_id,item.domain.module,item.captures.length]);
 if(!['org.junk.games','org.junk.music'].includes(item.domain.module)||item.captures.length===0)return null;
 const sources=item.captures.filter(capture=>!focusOperation||capture.association.item_id===item.id).flatMap(capture=>capture.representations.filter(representation=>!focusOperation||(representation.role==='preservation_master'&&(!focusSourceId||representation.id===focusSourceId))).map(representation=>({capture,representation}))).sort((left,right)=>Number(right.representation.id===focusSourceId)-Number(left.representation.id===focusSourceId));
 return <section className="import-workspace" aria-labelledby="processing-heading"><div className="section-heading"><div><h3 id="processing-heading">{music?'Music CD checks':'Game package checks'}</h3><p className="muted">{music?'CD structure, album identity and AccurateRip audio results answer different questions. The Album discs view summarizes them; these are the underlying checks.':'Check original files against a game catalog, inspect their structure, or compute checksums.'}</p></div></div>{!sources.length&&<p role="status">No current originals are available for this check.</p>}{sources.map(({capture,representation})=><RepresentationOperations key={`${representation.id}:${focusOperation||''}`} item={item} capture={capture} representation={representation} capabilities={capabilities} snapshots={snapshots} busy={busy} setBusy={setBusy} onError={onError} onSnapshotsChanged={loadSnapshots} focusOperation={focusOperation}/>)}</section>;
}

export default function ProcessingWorkspace(props:Props){
 return <ProcessingAnalysisWorkspace {...props}/>;
}
