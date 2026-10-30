import {useEffect,useRef,useState} from './runtime';
import type {operations} from './generated/api';

type BrowseQuery=NonNullable<operations['browse']['parameters']['query']>;
export type BrowseLocation={page:'purchases'|'lists'|'library'|'metadata'|'imports'|'preparation'|'activity';list:string;group:string;batch:string;reason:string;attention:boolean;collection:string;search:string;sort:NonNullable<BrowseQuery['sort']>;filter:NonNullable<BrowseQuery['filter']>;item:string};
const sorts=['title','recently_added','artist','platform'];
const filters=['all','wishlist','files_attached','playable','needs_attention','album_playback_needs_attention','removed'];
export function readLocation():BrowseLocation{
 const q=new URLSearchParams(location.search);
 return {list:q.get('list')||'',page:({'/purchase-inbox':'purchases','/lists':'lists','/library-care/metadata':'metadata','/imports':'imports','/library-care/music-preparation':'preparation','/activity':'activity'}[location.pathname]||'library') as BrowseLocation['page'],group:q.get('group')||'',batch:q.get('batch')||'',reason:q.get('reason')||'',attention:q.get('attention')!=='all',collection:q.get('collection')||'',search:q.get('search')||'',sort:(sorts.includes(q.get('sort')||'')?q.get('sort'):'title') as BrowseLocation['sort'],filter:(filters.includes(q.get('filter')||'')?q.get('filter'):'all') as BrowseLocation['filter'],item:q.get('item')||''};
}
export function browseUrl(value:BrowseLocation){const q=new URLSearchParams();if(value.page==='lists'&&value.list)q.set('list',value.list);if(value.collection)q.set('collection',value.collection);if(value.search)q.set('search',value.search);if(value.sort!=='title')q.set('sort',value.sort);if(value.filter!=='all')q.set('filter',value.filter);if(value.item)q.set('item',value.item);if(value.batch)q.set('batch',value.batch);if(value.group)q.set('group',value.group);if(value.page==='metadata'&&value.reason)q.set('reason',value.reason);if((value.page==='metadata'||value.page==='preparation')&&!value.attention)q.set('attention','all');return ({purchases:'/purchase-inbox',lists:'/lists',library:'/',metadata:'/library-care/metadata',imports:'/imports',preparation:'/library-care/music-preparation',activity:'/activity'}[value.page])+(q.size?'?'+q:'');}
export function useBrowseNavigation(){
 const [route,setRoute]=useState(readLocation);const [revision,setRevision]=useState(0);
 const workspaces=useRef(new Map<BrowseLocation['page'],{route:BrowseLocation;state:Record<string,unknown>}>());
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
  const switching=change.page&&change.page!==route.page;
  const remembered=switching?workspaces.current.get(change.page!):undefined;
  const saved=remembered?.route.collection===route.collection?remembered:undefined;
  const next={...route,...(saved?{...saved.route,collection:route.collection}:{}),...change};if(browseUrl(next)===location.pathname+location.search)return;const sameScope=browseUrl({...next,item:''})===browseUrl({...route,item:''});
  history.replaceState({...history.state,scroll:window.scrollY,focus:change.item||history.state?.focus},'',location.href);
  if(switching)workspaces.current.set(route.page,{route,state:history.state});
  const depth=next.item&&sameScope?(!route.item?1:(history.state?.depth?history.state.depth+(replace?0:1):0)):0;
  const returning=sameScope&&route.item&&!next.item;
  const state={pages:sameScope?(history.state?.pages||1):saved?.state.pages||1,scroll:returning?history.state?.return_scroll||0:saved?.state.scroll||0,focus:returning?route.item:saved?.state.focus,return_scroll:next.item&&sameScope?(route.item?history.state?.return_scroll:window.scrollY):undefined,depth};
  if(replace)history.replaceState(state,'',browseUrl(next));else history.pushState(state,'',browseUrl(next));
  restore.current={scroll:Number(state.scroll),focus:state.focus as string|undefined};setRoute(next);setRevision(v=>v+1);
 }
 function close(){if(route.item&&history.state?.depth)history.go(-history.state.depth);else navigate({item:''},true);}
 function setPages(pages:number){history.replaceState({...history.state,pages},'',location.href);setRevision(v=>v+1);}
 return {route,revision,navigate,close,setPages,restore,reset:()=>workspaces.current.clear(),pages:Number.isSafeInteger(history.state?.pages)&&history.state.pages>0?history.state.pages:1};
}
