// Which metadata matches the panel lists, which one it pins on top, what its
// status line claims, and which background lookup it reattaches to after a
// reload. metadata-review.tsx renders these; nothing here touches the page.
import type {components} from './generated/api';

type Review=components['schemas']['MetadataReviewView'];
type Candidate=components['schemas']['MetadataCandidateView'];
type CandidateIdentity=components['schemas']['MetadataCandidateIdentity'];
type Resolution=components['schemas']['MetadataResolutionView'];

export function identityKey(identity:CandidateIdentity){return JSON.stringify(identity);}
export function providerLabel(id:string){if(id==='org.junk.music.file-tags')return 'Imported tags';if(id==='org.junk.providers.musicbrainz')return 'MusicBrainz';if(id==='org.junk.providers.local-dat')return 'Game catalog';return id;}

// The match the entry's display comes from. A tentative identity is only a
// proposal, so it is never the one in use.
export function activeCandidate(review:Pick<Review,'display'|'candidates'>):Candidate|undefined{
 const shown=review.display.candidate;
 return review.display.resolution!=='tentative'&&shown?review.candidates.find(candidate=>identityKey(candidate.identity)===identityKey(shown)):undefined;
}

// The status line: imported tags, a provider match, manual metadata, or nothing.
export function metadataSourceLabel(active:Candidate|undefined,module:string,usesManualFields:boolean):string{
 if(active)return active.identity.provider==='org.junk.music.file-tags'?'Imported tags in use':`${providerLabel(active.identity.provider)} ${module==='org.junk.music'?'release':'match'} in use`;
 return usesManualFields?'Using manual metadata':'No match in use';
}

type ListOptions={module:string;showOthers:boolean;search:string;limit:number;pinned?:string};
// Music lists only the matches worth deciding between unless "show others" is
// on: chosen or automatically accepted ones, unmatched ones, and current
// strong (disc layout or identifier) matches. The search narrows that list by
// title, subtitle or record ID. The pinned match (the one in use, else the
// chosen one) goes first, then current matches before older ones, up to limit.
export function candidateList(candidates:readonly Candidate[],{module,showOthers,search,limit,pinned}:ListOptions){
 const listed=module==='org.junk.music'&&!showOthers?candidates.filter(candidate=>candidate.selected||candidate.accepted_automatically||candidate.unmatched||(candidate.current&&(candidate.strength==='disc_layout'||candidate.strength==='identifier'))):[...candidates];
 const query=search.trim().toLocaleLowerCase();
 const matching=query?listed.filter(candidate=>[candidate.title.value,candidate.subtitle?.value,candidate.identity.provider_record_id].some(value=>value?.toLocaleLowerCase().includes(query))):listed;
 const top=pinned?matching.filter(candidate=>identityKey(candidate.identity)===pinned):[];
 const others=matching.filter(candidate=>identityKey(candidate.identity)!==pinned).sort((left,right)=>Number(right.current)-Number(left.current));
 return {listed,hidden:candidates.length-listed.length,matching,shown:[...top,...others.slice(0,limit)]};
}

// After a reload, the panel follows a lookup still running, or else offers a
// retry for the newest lookup if it can be retried and wasn't dismissed.
export function resumableLookup(values:readonly Resolution[],dismissed:ReadonlySet<string>):Resolution|undefined{
 const running=new Set(['queued','running','cancel_requested']);
 return values.find(value=>running.has(value.job.state))??values.slice(0,1).find(value=>value.job.retryable&&!dismissed.has(value.job.id));
}
