import {reasonLabels,reasonLevels,type ReviewReason} from './review-reasons';
import {presentationLabel} from './presentation-labels';

// The server combines at most this many entries into one album in one request.
export const maxGroupedAlbums=32;

// Whether the open entry is waiting on a grouping decision. The entry's own
// review reasons decide this, so an entry opened from a link, another tab or a
// later list page behaves the same as one clicked in the loaded rows. Until
// those reasons arrive, the matching list row (if any) stands in for them.
export function proposesGrouping(reviewReasons:readonly ReviewReason[]|undefined,rowReasons:readonly ReviewReason[]|undefined=[]):boolean{
 return (reviewReasons??rowReasons??[]).includes('grouping_proposal');
}

type Selected={module:string;collection_id:string};

// Whether "Group into one album" can act on the selection: 2 to 32 music
// entries, all from one collection.
export function groupingAllowed(selection:readonly Selected[]):boolean{
 return selection.length>=2&&selection.length<=maxGroupedAlbums
  &&selection.every(item=>item.module==='org.junk.music')
  &&new Set(selection.map(item=>item.collection_id)).size===1;
}

// The Review lists and what each one asks the server for. "Needs you" lists
// entries that need a decision; Polish only polish-level reasons; All entries
// sends needs_attention=false, which the server reads as "don't filter on it".
export type ReviewTab='needs'|'progress'|'added'|'polish'|'all';
export const reviewTabs:ReviewTab[]=['needs','progress','added','polish','all'];
export const entryTabs:ReviewTab[]=['needs','polish','all'];
export function reviewTab(param:string):ReviewTab{return reviewTabs.includes(param as ReviewTab)?param as ReviewTab:'needs';}

type EntryScope={collection:string;search:string;reason:string;system:string;tab:ReviewTab};
export function entryListPath({collection,search,reason,system,tab}:EntryScope,cursor:string|null):string{
 const q=new URLSearchParams();if(collection)q.set('collection_id',collection);
 q.set('limit','50');if(search)q.set('search',search);if(reason)q.set('reason',reason);if(system)q.set('setup_system',system);
 if(tab==='polish'){q.set('needs_attention','false');q.set('level','polish');}else q.set('needs_attention',String(tab==='needs'));
 if(cursor)q.set('cursor',cursor);
 return `/library-care/metadata?${q}`;
}
// The same filters as entryListPath, for "Select all matching entries".
export function matchingEntriesQuery({collection,search,reason,system,tab}:EntryScope){
 return {collection_id:collection||null,search,reason:(reason||null) as ReviewReason|null,level:tab==='polish'?'polish' as const:null,needs_attention:tab==='needs',setup_system:system||null,cursor:null,limit:null};
}
export function ingestListPath({collection,intake}:{collection:string;intake:string},status:'active'|'attention'|'added',cursor:string|null):string{
 const q=new URLSearchParams();if(collection)q.set('collection_id',collection);
 q.set('status',status);q.set('limit','50');if(intake)q.set('intake_id',intake);if(cursor)q.set('cursor',cursor);
 return `/ingests?${q}`;
}

// The Reason filter offers only reasons the tab can show.
export function reasonFilterOptions(tab:ReviewTab):ReviewReason[]{
 return (Object.keys(reasonLabels) as ReviewReason[]).filter(value=>tab==='all'||(tab==='polish'?reasonLevels[value]==='polish':reasonLevels[value]!=='polish'&&reasonLevels[value]!=='setup'));
}

type PolledRow={summary_state:string;matching?:{state:string}|null};
// Rows still being summarized or matched refresh every 2 s; settled lists every 10 s.
export function entryPollDelay(rows:readonly PolledRow[]):number{
 return rows.some(row=>row.summary_state!=='ready'||!!row.matching&&['queued','running','cancel_requested'].includes(row.matching.state))?2000:10000;
}

export function reviewTabLabel(tab:ReviewTab,counts?:{needs_you:number;polish:number}):string{
 return tab==='needs'?`Needs you${counts?` ${counts.needs_you}`:''}`:tab==='progress'?'In progress':tab==='added'?'Added recently':tab==='all'?'All entries':`Polish${counts?` ${counts.polish}`:''}`;
}

type IngestState={permissions:{can_resume_upload:boolean};phase:string;outcome:string;capture_revision?:number|null};
// One import's state in plain words. A failed import says whether its
// originals were saved, because then nothing was lost.
export function ingestStateLabel(summary:IngestState):string{
 if(summary.permissions.can_resume_upload)return 'Waiting for files';
 if(summary.phase==='staging')return 'Saving originals';
 if(summary.phase==='analyzing'||summary.phase==='publishing')return 'Saving originals';
 if(summary.phase==='awaiting_review')return 'Needs a choice';
 if(summary.outcome==='failed')return summary.capture_revision!=null?'Couldn’t finish · originals saved':'Couldn’t import';
 if(summary.phase==='published'&&summary.outcome==='active')return 'Identifying';
 return presentationLabel(summary.outcome);
}

// "Next entry" opens the row after the open one; none at the end or when the
// open entry isn't in the list.
export function nextEntry<T extends {id:string}>(rows:readonly T[],itemId:string):T|undefined{
 const position=rows.findIndex(row=>row.id===itemId);
 return position>=0?rows[position+1]:undefined;
}

// In "all matching" mode every row is ticked except the exclusions.
export function rowChecked(row:{id:string},{all,exclusions,chosen}:{all:boolean;exclusions:readonly string[];chosen:readonly {id:string}[]}):boolean{
 return all?!exclusions.includes(row.id):chosen.some(value=>value.id===row.id);
}
