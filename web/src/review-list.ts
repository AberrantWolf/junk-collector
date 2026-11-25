import type {ReviewReason} from './review-reasons';

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
