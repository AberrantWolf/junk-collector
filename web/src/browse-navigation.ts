import type {ScrollTarget} from './browse-scroll-restore';
import {useEffect,useRef,useState} from './runtime';
import {browseUrl,readLocation,type BrowseLocation} from './browse-location';
export {browseUrl,readLocation,type BrowseLocation};

export function useBrowseNavigation(){
 const [route,setRoute]=useState(readLocation);const [revision,setRevision]=useState(0);
 const workspaces=useRef(new Map<BrowseLocation['page'],{route:BrowseLocation;state:Record<string,unknown>}>());
 const restore=useRef<ScrollTarget|null>(null);
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
