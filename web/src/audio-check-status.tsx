import React from '@collector/runtime';
import type {components} from './generated/api';
type Item=components['schemas']['PresentedItem'];
const labels={verified:'Verified',incomplete_search:'Incomplete search',mismatched:'Mismatch',no_data:'No data',ambiguous_offsets:'Ambiguous offsets',unavailable:'Unavailable',unsupported:'Unsupported'};

export default function AudioCheckStatus({item,onOpen}:{item:Pick<Item,'title'|'digital_audio'|'accuraterip'>;onOpen?:()=>void}){
 if(item.digital_audio&&!item.accuraterip?.length)return <span className="audio-check-status" title="Saved digital purchase; CD verification does not apply">Digital original</span>;
 const checks=item.accuraterip||[];
 const verified=checks.filter(check=>check.status==='verified').length;
 const unchecked=checks.filter(check=>!check.status).length;
 const mismatch=checks.some(check=>check.status==='mismatched');
 const sameStatus=checks.length>0&&checks.every(check=>check.status===checks[0].status);
 const label=!checks.length?'No originals':unchecked===checks.length?'Not checked':mismatch?(verified?`${verified}/${checks.length} · mismatch`:'Mismatch'):checks.length===1?labels[checks[0].status!]:verified?`${verified}/${checks.length} verified`:sameStatus?labels[checks[0].status!]:'Inconclusive';
 const description=checks.length?checks.map((check,index)=>`Original ${index+1}: ${check.status?labels[check.status]:'Not checked'}`).join('; '):'No saved originals available to check';
 const title=`${description}. Results cover these originals; album disc coverage is separate.`;
 return onOpen&&checks.length?<button className="audio-check-status" title={title} aria-label={`AccurateRip for ${item.title}: ${label}. Open audio checks`} onClick={onOpen}>{label}</button>:<span className="audio-check-status" title={title}>{label}</span>;
}
