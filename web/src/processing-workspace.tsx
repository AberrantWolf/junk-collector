import React,{useEffect,useState,api} from '@collector/runtime';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type Capture=components['schemas']['PresentedCapture'];
type Representation=components['schemas']['Representation'];
type Capability=components['schemas']['CapabilityAffordance'];
type SnapshotPage=components['schemas']['Page_CatalogSnapshotStatus'];
type SnapshotStatus=components['schemas']['CatalogSnapshotStatus'];
type SummaryPage=components['schemas']['Page_ProcessingSummary'];
type Summary=components['schemas']['ProcessingSummary'];
type JobView=components['schemas']['JobView'];
type Props={item:Item;busy:boolean;setBusy:(value:boolean)=>void;onError:(error:unknown)=>void};

const terminal=new Set<JobView['state']>(['completed','failed','cancelled']);
const operations:Record<string,string>={
 'org.junk.games.hash':'Hash game data',
 'org.junk.games.inspect-ines':'Inspect iNES structure',
 'org.junk.games.hash-tracks':'Hash raw 2352-byte tracks',
 'org.junk.games.catalog-verification':'Compare with a DAT snapshot',
 'org.junk.music.cd-structure':'Inspect CD structure',
 'org.junk.music.accuraterip-verification':'Verify audio with AccurateRip',
};

function object(value:unknown):Record<string,unknown>{return value!==null&&typeof value==='object'&&!Array.isArray(value)?value as Record<string,unknown>:{};}
function snapshotLabel(status:SnapshotStatus){const value=object(status.snapshot.parsed_observation);return `${String(value.catalog||'DAT catalog')} ${String(value.version||'unversioned')} · ${String(value.system||'unknown system')}${status.active?' · active':''}`;}
function musicSnapshotLabel(status:SnapshotStatus){const value=object(status.snapshot.parsed_observation);const query=object(value.query);return `AccurateRip ${String(value.outcome||'response')} · ${String(query.track_count||'?')} tracks · ${status.snapshot.id}`;}
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
 if(result.result==='audio_verification'){const verification=object(result.verification);return `AccurateRip: ${String(verification.status||'unknown').replaceAll('_',' ')}`;}
 if(Array.isArray(result.observations))return result.observations.map(value=>resultLabel({...summary,result:object(value)})).join(' · ');
 return 'Processing observation';
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
     Track {String(track.position)}: {String(track.status).replaceAll('_',' ')}, computed at {String(track.computed_offset)} stereo sample frames.
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

function RepresentationOperations({item,capture,representation,capabilities,snapshots,busy,setBusy,onError,onSnapshotsChanged}:{item:Item;capture:Capture;representation:Representation;capabilities:Capability[];snapshots:SnapshotStatus[];busy:boolean;setBusy:(value:boolean)=>void;onError:(error:unknown)=>void;onSnapshotsChanged:()=>Promise<void>}){
 const available=capabilities.filter(value=>operations[value.id]);
 const [operation,setOperation]=useState(available.find(value=>value.availability.state==='available')?.id||'');
 const [fileIndex,setFileIndex]=useState(0);const [packageKind,setPackageKind]=useState('cue');const [maximumOffset,setMaximumOffset]=useState(2939);const [profile,setProfile]=useState('raw');const [copierHeader,setCopierHeader]=useState(false);const [catalogInput,setCatalogInput]=useState('raw_files');const [system,setSystem]=useState(()=>String(object(item.domain.payload).platform||''));const [snapshotId,setSnapshotId]=useState(()=>snapshots.find(value=>value.active)?.snapshot.id||snapshots[0]?.snapshot.id||'');const [summaries,setSummaries]=useState<Summary[]>([]);const [cursor,setCursor]=useState<string|null>(null);
 async function load(after?:string){const page=await api<SummaryPage>(`/captures/${capture.id}/operations?representation_id=${representation.id}&limit=100${after?`&cursor=${encodeURIComponent(after)}`:''}`);setSummaries(current=>after?[...current,...page.items]:page.items);setCursor(page.next_cursor??null);}
 useEffect(()=>{load().catch(onError);},[capture.id,representation.id]);
 useEffect(()=>{setSnapshotId(current=>current&&snapshots.some(value=>value.snapshot.id===current)?current:snapshots.find(value=>value.active)?.snapshot.id||snapshots[0]?.snapshot.id||'');},[snapshots]);
 async function run(){
  const selected=capabilities.find(value=>value.id===operation);if(!selected||selected.availability.state!=='available')return;
  let selection:Record<string,unknown>;let frozenSnapshot:string|null=null;
  if(operation==='org.junk.games.catalog-verification'){
   const analysis=catalogInput==='raw_files'?{input:'raw_files'}:catalogInput==='cue_raw2352'?{input:'cue_raw2352',cue_file_index:fileIndex}:{input:'file',file_index:fileIndex,profile,...(['snes_payload','pce_payload'].includes(profile)?{copier_header:copierHeader}:{})};
   selection={snapshot_id:snapshotId,system,...analysis};frozenSnapshot=snapshotId;
  }
  else if(operation==='org.junk.music.cd-structure')selection={package_kind:packageKind,anchor_file_index:fileIndex};
  else if(operation==='org.junk.music.accuraterip-verification'){selection={package_kind:packageKind,anchor_file_index:fileIndex,snapshot_id:snapshotId,maximum_offset:maximumOffset};frozenSnapshot=snapshotId;}
  else if(operation==='org.junk.games.hash-tracks')selection={cue_file_index:fileIndex};
  else if(operation==='org.junk.games.inspect-ines')selection={file_index:fileIndex};
  else selection={file_index:fileIndex,profile,...(['snes_payload','pce_payload'].includes(profile)?{copier_header:copierHeader}:{})};
  setBusy(true);
  try{
   const accepted=await api<{job_id:string}>(`/captures/${capture.id}/operations`,{method:'POST',body:JSON.stringify({operation,operation_version:1,representation_id:representation.id,snapshot_id:frozenSnapshot,selection})});
   for(;;){const job=await api<JobView>(`/jobs/${accepted.job_id}`);if(terminal.has(job.state)){if(job.state!=='completed')throw new Error(job.error||job.limitation||`Operation ${job.state}.`);break;}await new Promise(resolve=>setTimeout(resolve,1200));}
   await load();window.dispatchEvent(new Event('collector-jobs-updated'));
  }catch(error){onError(error);}finally{setBusy(false);}
 }
 async function acquire(){const structure=summaries.find(summary=>summary.current&&object(summary.result).result==='layout');if(!structure)return;setBusy(true);try{const accepted=await api<{job_id:string}>(`/captures/${capture.id}/accuraterip-snapshots`,{method:'POST',body:JSON.stringify({representation_id:representation.id,structure_observation_id:structure.observation_id})});for(;;){const job=await api<JobView>(`/jobs/${accepted.job_id}`);if(terminal.has(job.state)){if(job.state!=='completed')throw new Error(job.error||job.limitation||`AccurateRip request ${job.state}. Preservation and structural evidence remain available.`);break;}await new Promise(resolve=>setTimeout(resolve,1200));}await onSnapshotsChanged();window.dispatchEvent(new Event('collector-jobs-updated'));}catch(error){onError(error);}finally{setBusy(false);}}
 const selected=capabilities.find(value=>value.id===operation);const unavailable=selected?.availability.state==='unavailable'?selected.availability.limitation:'';
 return <div className="import-docket"><header><div><span className="docket-phase">{representation.role.replaceAll('_',' ')}</span><strong className="docket-title">{representation.format||'Original package'} · {representation.files.length} files</strong></div></header>
  <div className="form-grid"><label>Operation<select value={operation} disabled={busy} onChange={event=>setOperation(event.target.value)}>{available.map(value=><option key={value.id} value={value.id}>{operations[value.id]}{value.availability.state==='unavailable'?' (unavailable)':''}</option>)}</select></label><label>Package file<select value={fileIndex} disabled={busy} onChange={event=>setFileIndex(Number(event.target.value))}>{representation.files.map((file,index)=><option key={index} value={index}>{index}: {file.name}</option>)}</select></label></div>
  {operation==='org.junk.games.hash'&&<><label>Explicit byte-domain profile<select value={profile} disabled={busy} onChange={event=>setProfile(event.target.value)}><option value="raw">Raw file bytes</option><option value="ines_payload">iNES payload</option><option value="n64_big_endian">Nintendo 64 big-endian</option><option value="snes_payload">SNES payload</option><option value="pce_payload">PC Engine payload</option></select></label>{['snes_payload','pce_payload'].includes(profile)&&<label><input type="checkbox" checked={copierHeader} disabled={busy} onChange={event=>setCopierHeader(event.target.checked)}/> Source includes an explicit 512-byte copier header</label>}</>}
  {operation==='org.junk.games.catalog-verification'&&<><div className="form-grid"><label>Explicit DAT system<input value={system} disabled={busy} onChange={event=>setSystem(event.target.value)}/></label><label>Immutable snapshot<select value={snapshotId} disabled={busy} onChange={event=>setSnapshotId(event.target.value)}>{snapshots.map(value=><option key={value.snapshot.id} value={value.snapshot.id}>{snapshotLabel(value)}</option>)}</select></label></div><div className="form-grid"><label>Catalog byte input<select value={catalogInput} disabled={busy} onChange={event=>setCatalogInput(event.target.value)}><option value="raw_files">All ordered raw files</option><option value="file">One normalized file profile</option><option value="cue_raw2352">All ordered raw-2352 CUE tracks</option></select></label>{catalogInput==='file'&&<label>Explicit byte-domain profile<select value={profile} disabled={busy} onChange={event=>setProfile(event.target.value)}><option value="raw">Raw file bytes</option><option value="ines_payload">iNES payload</option><option value="n64_big_endian">Nintendo 64 big-endian</option><option value="snes_payload">SNES payload</option><option value="pce_payload">PC Engine payload</option></select></label>}</div>{catalogInput==='file'&&['snes_payload','pce_payload'].includes(profile)&&<label><input type="checkbox" checked={copierHeader} disabled={busy} onChange={event=>setCopierHeader(event.target.checked)}/> Source includes an explicit 512-byte copier header</label>}</>}
  {operation==='org.junk.games.catalog-verification'&&snapshots.length===0&&<p className="notice">No DAT snapshot is available. Hashing and structural inspection remain usable.</p>}
  {operation.startsWith('org.junk.music.')&&<label>Package layout<select value={packageKind} disabled={busy} onChange={event=>setPackageKind(event.target.value)}><option value="cue">BIN/CUE</option><option value="redumper">Current raw redumper</option></select></label>}
  {operation==='org.junk.music.accuraterip-verification'&&<><div className="form-grid"><label>Frozen provider snapshot<select value={snapshotId} disabled={busy} onChange={event=>setSnapshotId(event.target.value)}>{snapshots.map(value=><option key={value.snapshot.id} value={value.snapshot.id}>{musicSnapshotLabel(value)}</option>)}</select></label><label>Maximum offset in stereo sample frames<input type="number" min="0" max="2939" value={maximumOffset} disabled={busy} onChange={event=>setMaximumOffset(Number(event.target.value))}/></label></div>{snapshots.length===0&&<p className="notice">Fetch a response after recording a supported CD structure. No-data and provider failures remain distinct from an audio mismatch.</p>}</>}
  {unavailable&&<p className="notice">{unavailable}</p>}<div className="button-row"><button disabled={busy||!operation||!!unavailable||(operation==='org.junk.games.catalog-verification'&&(!system||!snapshotId))||(operation==='org.junk.music.accuraterip-verification'&&!snapshotId)} onClick={()=>void run()}>Run operation</button>{item.domain.module==='org.junk.music'&&summaries.some(summary=>summary.current&&object(summary.result).result==='layout')&&<button disabled={busy} onClick={()=>void acquire()}>Fetch AccurateRip response</button>}</div>
  {summaries.length>0&&<div><h4>Processing history</h4>{summaries.map(summary=>{const snapshot=snapshots.find(value=>value.snapshot.id===summary.snapshot_id);return <div className="stored-file" key={summary.observation_id}><span><strong>{resultLabel(summary)}</strong><small>{summary.current?'Current recorded source':'Historical recorded source'} · {summary.operation}<br/>{snapshot?(item.domain.module==='org.junk.music'?musicSnapshotLabel(snapshot):snapshotLabel(snapshot)):summary.snapshot_id?`Snapshot ${summary.snapshot_id}`:'No provider snapshot'}{summary.limitations.length?` · ${summary.limitations.join(' · ')}`:''}</small><ResultDetails summary={summary}/></span></div>;})}{cursor&&<button disabled={busy} onClick={()=>void load(cursor).catch(onError)}>Load older observations</button>}</div>}
 </div>;
}

export default function ProcessingWorkspace({item,busy,setBusy,onError}:Props){
 const [snapshots,setSnapshots]=useState<SnapshotStatus[]>([]);const capabilities=item.presentation.capabilities.filter(value=>value.id.startsWith(`${item.domain.module}.`));const music=item.domain.module==='org.junk.music';
 async function loadSnapshots(){const page=await api<SnapshotPage>(`/collections/${item.collection_id}/catalog-snapshots?limit=100`);setSnapshots(page.items.filter(value=>music?value.snapshot.provider==='org.junk.providers.accuraterip':value.snapshot.provider==='org.junk.providers.local-dat'));}
 useEffect(()=>{if(!['org.junk.games','org.junk.music'].includes(item.domain.module)||item.captures.length===0)return;loadSnapshots().catch(onError);},[item.collection_id,item.domain.module,item.captures.length]);
 if(!['org.junk.games','org.junk.music'].includes(item.domain.module)||item.captures.length===0)return null;
 return <section className="import-workspace" aria-labelledby="processing-heading"><div className="section-heading"><div><h3 id="processing-heading">{music?'Music CD analysis':'Game package analysis'}</h3><p className="muted">{music?'Record structure separately, then optionally fetch and verify against an immutable AccurateRip response. Provider failure never blocks preservation.':'Run explicit, versioned operations against preserved package bytes. Results remain tied to the exact source and catalog snapshot.'}</p></div></div>{capabilities.map(value=><p className="muted" key={value.id}>{operations[value.id]||value.label}: {value.availability.state==='available'?'Available':`Unavailable — ${value.availability.limitation}`}</p>)}{item.captures.flatMap(capture=>capture.representations.map(representation=><RepresentationOperations key={representation.id} item={item} capture={capture} representation={representation} capabilities={capabilities} snapshots={snapshots} busy={busy} setBusy={setBusy} onError={onError} onSnapshotsChanged={loadSnapshots}/>))}</section>;
}
