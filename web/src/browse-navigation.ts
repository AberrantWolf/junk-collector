import {useEffect,useRef,useState} from './runtime';
import type {operations} from './generated/api';

type BrowseQuery=NonNullable<operations['browse']['parameters']['query']>;
export type BrowseLocation={collection:string;search:string;sort:NonNullable<BrowseQuery['sort']>;filter:NonNullable<BrowseQuery['filter']>;item:string};
const sorts=['title','recently_added','artist','platform'];
const filters=['all','wishlist','files_attached','playable','needs_attention','removed'];
export function readLocation():BrowseLocation{
 const q=new URLSearchParams(location.search);
 return {collection:q.get('collection')||'',search:q.get('search')||'',sort:(sorts.includes(q.get('sort')||'')?q.get('sort'):'title') as BrowseLocation['sort'],filter:(filters.includes(q.get('filter')||'')?q.get('filter'):'all') as BrowseLocation['filter'],item:q.get('item')||''};
}
export function browseUrl(value:BrowseLocation){const q=new URLSearchParams();if(value.collection)q.set('collection',value.collection);if(value.search)q.set('search',value.search);if(value.sort!=='title')q.set('sort',value.sort);if(value.filter!=='all')q.set('filter',value.filter);if(value.item)q.set('item',value.item);return '/'+(q.size?'?'+q:'');}
export function useBrowseNavigation(){
 const [route,setRoute]=useState(readLocation);const [revision,setRevision]=useState(0);
 const restore=useRef<{scroll:number;focus?:string}|null>(null);
 useEffect(()=>{const previous=history.scrollRestoration;history.scrollRestoration='manual';
  const pop=()=>{restore.current={scroll:history.state?.scroll||0,focus:history.state?.focus};setRoute(readLocation());setRevision(v=>v+1);};
  const save=()=>history.replaceState({...history.state,scroll:window.scrollY},'',location.href);
  window.addEventListener('popstate',pop);window.addEventListener('pagehide',save);
  // History state survives a reload without storing any collection contents.
  restore.current={scroll:history.state?.scroll||0,focus:history.state?.focus};
  return()=>{history.scrollRestoration=previous;window.removeEventListener('popstate',pop);window.removeEventListener('pagehide',save);};
 },[]);
 function navigate(change:Partial<BrowseLocation>,replace=false){
  const next={...route,...change};if(browseUrl(next)===location.pathname+location.search)return;const sameScope=browseUrl({...next,item:''})===browseUrl({...route,item:''});
  history.replaceState({...history.state,scroll:window.scrollY,focus:change.item||history.state?.focus},'',location.href);
  const depth=next.item&&sameScope?(!route.item?1:(history.state?.depth?history.state.depth+(replace?0:1):0)):0;
  const state={pages:sameScope?(history.state?.pages||1):1,scroll:0,depth};
  if(replace)history.replaceState(state,'',browseUrl(next));else history.pushState(state,'',browseUrl(next));
  restore.current={scroll:0};setRoute(next);setRevision(v=>v+1);
 }
 function close(){if(route.item&&history.state?.depth)history.go(-history.state.depth);else navigate({item:''},true);}
 function setPages(pages:number){history.replaceState({...history.state,pages},'',location.href);setRevision(v=>v+1);}
 return {route,revision,navigate,close,setPages,restore,pages:Number.isSafeInteger(history.state?.pages)&&history.state.pages>0?history.state.pages:1};
}
