// Rules for an import review: how packages are sent in batches, what import
// intent the choices build, which packages need a decision, and how a folder's
// layout is guessed. library-imports.tsx holds the state and sends requests.
import type {components} from './generated/api';
import {audioFormat} from './import-file-order';
import {importPreparation,type ImportPreparationChoice} from './preparation-presets';

type S=components['schemas'];

// The server accepts at most 64 packages, 10,000 files or 12 MiB of request
// per /intakes/{id}/members call.
export const intakeBatchLimits={packages:64,files:10000,bytes:12*1024*1024};

// Splits packages from `resumeFrom` on into request batches. A package that is
// over a limit on its own still goes alone, so the split always moves on.
export function intakeBatches<T>(packages:readonly T[],resumeFrom:number,measure:(entry:T)=>{files:number;bytes:number},limits=intakeBatchLimits):{start:number;end:number}[]{
 const batches:{start:number;end:number}[]=[];
 for(let start=resumeFrom;start<packages.length;){
  let end=start,files=0,bytes=0;
  while(end<packages.length&&end-start<limits.packages){
   const size=measure(packages[end]);
   if(end>start&&(files+size.files>limits.files||bytes+size.bytes>limits.bytes))break;
   files+=size.files;bytes+=size.bytes;end++;
  }
  batches.push({start,end});start=end;
 }
 return batches;
}

// The import intent the review's choices build. "Preserve only" keeps the
// originals and turns off identification and audio preparation.
export function intakeIntent({automatic,preserveOnly,music,audioChoice,opusDefault}:{automatic:boolean;preserveOnly:boolean;music:boolean;audioChoice:ImportPreparationChoice;opusDefault?:S['OpusImportDefault']}){
 return {
  automatic:automatic&&!preserveOnly,prepare:false,
  analysis:preserveOnly?'preserve_only' as const:'automatic' as const,
  audio_preparation:music&&!preserveOnly?importPreparation(audioChoice):undefined,
  reviewed_opus_import:music&&audioChoice==='default'&&!preserveOnly?opusDefault:undefined,
 };
}

type Reviewed={package:{album_tracks?:readonly unknown[]|null};destination:{destination:string;copy_key?:string}};
type Planned={blockers:readonly unknown[];music_layout?:{needs_attention:boolean}|null};
// A package is shown for a decision when the batch is small (10 or fewer),
// the curator opened it, its plan has blockers, its album tracks need review,
// or it joins an album without naming the copy it came from.
export function needsDecision(entry:Reviewed,plan:Planned|undefined,{total,expanded}:{total:number;expanded:boolean}):boolean{
 return total<=10||expanded||!!plan?.blockers.length
  ||(entry.package.album_tracks?!entry.package.album_tracks.length:!!plan?.music_layout?.needs_attention)
  ||entry.destination.destination!=='separate'&&!entry.destination.copy_key?.trim();
}

// Audio files without a CUE or TOC are a digital purchase, not a disc rip.
export function isDigitalPackage(names:readonly string[]):boolean{
 return names.some(name=>!!audioFormat(name))&&!names.some(name=>/\.(cue|toc)$/i.test(name));
}

// Whether a chosen folder is one album rather than a folder of albums: audio
// directly inside it, or inside a format folder (FLAC, WAV, MP3) or a disc
// folder (Disc 1, CD-2) one level down.
export function albumRoot(relativePaths:readonly string[]):boolean{
 return relativePaths.some(path=>{
  const parts=path.split('/');
  if(!/\.(flac|wav|mp3)$/i.test(parts.at(-1)||''))return false;
  return parts.length===2||/^(?:flac|wav|mp3|(?:disc|cd)[ _-]*[0-9]+)$/i.test(parts[1]||'');
 });
}
