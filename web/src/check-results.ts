// The sentence a curator reads for one recorded check: an integrity hash, a
// structure inspection, a catalog match or an AccurateRip result. These stay
// separate claims, and a label never says more than the check computed: a
// partial or ambiguous match never reads as a full one, and a CRC32-only match
// says it is not cryptographic identity.
import {presentationLabel} from './presentation-labels';

function object(value:unknown):Record<string,unknown>{return value&&typeof value==='object'&&!Array.isArray(value)?value as Record<string,unknown>:{};}

export function resultLabel(summary:{result:unknown}):string{
 const result=object(summary.result);const catalog=object(result.catalog);const kind=String(catalog.match||'');
 const candidates=kind==='unique_complete_match'?[object(catalog.candidate)]:Array.isArray(catalog.candidates)?catalog.candidates.map(object):[];
 const matched=candidates.flatMap(candidate=>Array.isArray(candidate.matched_units)?candidate.matched_units.map(object):[]);
 const crcOnly=matched.length>0&&matched.every(unit=>object(unit.digest).algorithm==='crc32');
 const checksumNote=crcOnly?' (CRC32 only; not cryptographic identity)':'';
 if(kind==='unique_complete_match')return `Exact catalog match${checksumNote}`;
 if(kind==='ambiguous_complete_matches')return `Exact but ambiguous catalog matches${checksumNote}`;
 if(kind==='partial_identification')return `Partial identification only${checksumNote}`;
 if(kind==='no_match')return 'No catalog match';
 if(kind==='inconclusive')return `Inconclusive: ${String(catalog.reason||'not enough information')}`;
 if(Array.isArray(object(result.file_tracks).tracks))return `Digital audio analyzed · ${(object(result.file_tracks).tracks as unknown[]).length} tracks`;
 if(result.result==='hashes')return 'File hashes recorded';
 if(result.result==='track_hashes')return 'Track hashes recorded';
 if(result.result==='structure')return `${String(result.format||'Game')} structure inspected`;
 if(result.result==='cd_audio')return `Game CD audio recorded · ${Array.isArray(object(result.audio).tracks)?(object(result.audio).tracks as unknown[]).length:0} audio tracks`;
 if(result.result==='layout')return `CD structure recorded · ${Array.isArray(result.tracks)?result.tracks.length:0} tracks`;
 if(result.result==='audio_verification'){const verification=object(result.verification);return `AccurateRip: ${presentationLabel(String(verification.status||'unknown'))}`;}
 if(Array.isArray(result.observations))return result.observations.map(value=>resultLabel({...summary,result:object(value)})).join(' · ');
 return 'Recorded check';
}
