import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage,setRecentAuthenticationHandler,setApiIdentity,useOperationScope} from './runtime';
import {createRoot} from 'react-dom/client';
import type {components} from './generated/api';
import MusicTracks from './music-tracks';
import MusicPlayer from './music-player';
import {setMusicQueueUser,addMusicTracks,audioMediaType} from './music-queue';
import {useBrowseNavigation,type BrowseLocation} from './browse-navigation';

export {useMusicQueue,addMusicTracks} from './runtime';
export {OperationScope,useOperationScope,setApiIdentity,default,useState,useEffect,useRef,ApiRequestError,isApiRequestError,requestErrorMessage,api} from './runtime';

type Collection=components['schemas']['PresentedCollection'];
type CollectionPageState={key:string;items:Collection[];cursor:string|null;ready:boolean;loading:boolean;error:string};
type Features=components['schemas']['FeaturesResponse'];
type Item=components['schemas']['PresentedItem'];
type Page=components['schemas']['Page_PresentedItem'];
type Principal=components['schemas']['Principal'];
type CollectionType=components['schemas']['ModuleDescriptor'];

async function usePasskey(username:string,signal?:AbortSignal):Promise<Principal>{
 const started=await api<{ceremony_id:string;options:{publicKey:PublicKeyCredentialRequestOptionsJSON}}>('/auth/passkeys/login/start',{method:'POST',signal,body:JSON.stringify({username})});
 const publicKey=PublicKeyCredential.parseRequestOptionsFromJSON(started.options.publicKey);
 const credential=await navigator.credentials.get({publicKey,signal}) as PublicKeyCredential|null;
 if(!credential)throw new Error('Passkey sign-in was cancelled.');
 const result=await api<{principal:Principal}>('/auth/passkeys/login/finish',{method:'POST',signal,body:JSON.stringify({ceremony_id:started.ceremony_id,credential:credential.toJSON()})});return result.principal;
}

async function registerPasskey(){
 const started=await api<{ceremony_id:string;options:{publicKey:PublicKeyCredentialCreationOptionsJSON}}>('/auth/passkeys/register/start',{method:'POST'});
 const publicKey=PublicKeyCredential.parseCreationOptionsFromJSON(started.options.publicKey);
 const credential=await navigator.credentials.create({publicKey}) as PublicKeyCredential|null;
 if(!credential)throw new Error('Passkey registration was cancelled.');
 await api('/auth/passkeys/register/finish',{method:'POST',body:JSON.stringify({ceremony_id:started.ceremony_id,credential:credential.toJSON()})});
}

function Artwork({item,large=false}:{item:Item;large?:boolean}){
 const artwork=item.assets.find(asset=>asset.id===item.primary_artwork);const [failed,setFailed]=useState(false);const [attempt,setAttempt]=useState(0);
 useEffect(()=>{setFailed(false);setAttempt(0);},[artwork?.id,artwork?.revision]);
 return <div className="artwork-stack"><div className={`artwork ${large?'large':''}`}>{artwork?(failed?<span className="preview-failed"><small>Preview unavailable</small>{large&&<button onClick={()=>{setFailed(false);setAttempt(v=>v+1);}}>Retry artwork</button>}</span>:<img key={attempt} src={`/api/v1/collections/${item.collection_id}/assets/${artwork.id}/thumbnail?retry=${attempt}`} loading="lazy" alt={artwork.alt_text||`${item.title} artwork`} onError={()=>setFailed(true)}/>):<span role="img" aria-label="No artwork attached" className="art-placeholder"><b aria-hidden="true">◇</b><small>No art</small></span>}</div>{item.effective_metadata.resolution==='tentative'&&<small className="metadata-state">Tentative metadata</small>}</div>;
}

function App(){
 const navigation=useBrowseNavigation();const {route,navigate,close:closeDetails}=navigation;const {collection,search,sort,filter}=route;
 const [detailError,setDetailError]=useState('');const [detailLoading,setDetailLoading]=useState(false);const [detailRetry,setDetailRetry]=useState(0);const [listFailed,setListFailed]=useState(false);
 const detailsRef=useRef<HTMLElement>(null);const listScope=useRef('');
 const pageCache=useRef<{scope:string;refresh:number;items:Item[];cursor:string|null;pages:number}|undefined>(undefined);
 function changeScope(change:Partial<BrowseLocation>,replace=false){managementRequest.current?.abort();setSelected(undefined);setError('');navigate({...change,item:''},replace);}
 const [features,setFeatures]=useState<Features>();const [actor,setActorState]=useState<Principal>();const [ready,setReady]=useState(false);
 const [collectionPage,setCollectionPage]=useState<CollectionPageState>({key:'',items:[],cursor:null,ready:false,loading:true,error:''});const [collectionSearch,setCollectionSearch]=useState('');const [collectionRevision,setCollectionRevision]=useState(0);const [selectedCollectionExact,setSelectedCollectionExact]=useState<Collection>();const [collectionTypes,setCollectionTypes]=useState<CollectionType[]>([]);const [items,setItems]=useState<Item[]>([]);const [cursor,setCursor]=useState<string|null>(null);const [selected,setSelected]=useState<Item>();const [error,setError]=useState('');const [loginMessage,setLoginMessage]=useState('');const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false);const [refresh,setRefresh]=useState(0);
 const [canCurateAny,setCanCurateAny]=useState(false);const [canManageAny,setCanManageAny]=useState(false);
 const collectionKey=JSON.stringify([actor?.user_id,collectionSearch,collectionRevision]);const collectionReads=useOperationScope(collectionKey);const collectionCurrent=collectionPage.key===collectionKey;const collections=collectionCurrent?collectionPage.items:[];const collectionCursor=collectionCurrent?collectionPage.cursor:null;const collectionsReady=collectionCurrent&&collectionPage.ready;const collectionLoading=!collectionCurrent||collectionPage.loading;const collectionError=collectionCurrent?collectionPage.error:'';
 useEffect(()=>{function escape(event:KeyboardEvent){if(event.key==='Escape'&&!document.querySelector('[role="dialog"]')&&route.item){managementRequest.current?.abort();closeDetails();}}document.addEventListener('keydown',escape);return()=>document.removeEventListener('keydown',escape);},[route.item]);
 useEffect(()=>{setMusicQueueUser(actor?.user_id);return()=>setMusicQueueUser(undefined);},[actor?.user_id]);
 function setActor(value?:Principal){setApiIdentity(value);setActorState(value);}
 const [reauth,setReauth]=useState<number|null>(null);const reauthGeneration=useRef(0);const [reauthUsername,setReauthUsername]=useState('');const reauthSettlement=useRef<{id:number;userId:string;resolve:(identity:Principal)=>void;reject:(error:Error)=>void}|null>(null);
 useEffect(()=>{setRecentAuthenticationHandler(async({identity,signal})=>{
  const id=++reauthGeneration.current;const profile=await api<{username:string}>('/me/profile',{signal});
  signal.throwIfAborted();if(id!==reauthGeneration.current)throw new Error('Reauthentication cancelled.');setReauthUsername(profile.username);
  return new Promise<Principal>((resolve,reject)=>{
   const abort=()=>finishReauth(undefined,id);
   signal.addEventListener('abort',abort,{once:true});
   reauthSettlement.current={id,userId:identity.user_id,resolve:value=>{signal.removeEventListener('abort',abort);if(value.user_id!==identity.user_id)reject(new Error('Account changed. Review the pending action.'));else resolve(value);},reject:error=>{signal.removeEventListener('abort',abort);reject(error);}};setReauth(id);
  });
 });return()=>setRecentAuthenticationHandler();},[]);
 function finishReauth(value:Principal|undefined,id:number){const settlement=reauthSettlement.current;if(settlement?.id!==id)return;reauthSettlement.current=null;reauthGeneration.current++;setReauth(null);if(value){if(value.user_id!==settlement.userId){settlement.reject(new Error('Account changed. The pending action was not submitted.'));clearSession('The authenticated account changed. Sign in again to load its collections.');return;}setActor(value);settlement.resolve(value);}else settlement.reject(new Error('Reauthentication cancelled. Your draft remains available; submit it again when ready.'));}
 const [management,setManagement]=useState<React.ReactNode>(null);const generation=useRef(0);const managementRequest=useRef<AbortController|null>(null);

 function clearSession(message:string){
  setApiIdentity(undefined);if(reauthSettlement.current)finishReauth(undefined,reauthSettlement.current.id);else reauthGeneration.current++;setMusicQueueUser(undefined);
  managementRequest.current?.abort();collectionReads.cancel();openedWorkLink.current=false;generation.current++;pageCache.current=undefined;setBusy(false);setItems([]);setCollectionPage({key:'',items:[],cursor:null,ready:false,loading:true,error:''});setSelectedCollectionExact(undefined);setCanCurateAny(false);setCanManageAny(false);setCollectionTypes([]);setSelected(undefined);setManagement(null);setError('');setNotice('');setLoginMessage(message);setActor(undefined);
 }
 function handleFailure(value:unknown,forbiddenMessage?:string){
  if(isApiRequestError(value)&&value.code==='unauthenticated'){
   clearSession('Your session expired or was revoked. Sign in again; the failed operation was not saved.');return;
  }
  setError(requestErrorMessage(value,forbiddenMessage));
 }

 useEffect(()=>{
  api<Features>('/features').then(setFeatures).catch(e=>setError(requestErrorMessage(e)));
  api<Principal>('/me').then(value=>{setActor(value);setLoginMessage('');}).catch(e=>{if(!isApiRequestError(e)||e.code!=='unauthenticated')setError(requestErrorMessage(e));}).finally(()=>setReady(true));
 },[]);
 useEffect(()=>{if(!actor)return;collectionReads.cancel();setCollectionPage(state=>state.key===collectionKey?{...state,loading:true,error:''}:{key:collectionKey,items:[],cursor:null,ready:false,loading:true,error:''});const timer=setTimeout(()=>{const query=new URLSearchParams({search:collectionSearch,limit:'50'});void collectionReads.run(attempt=>api<{items:Collection[];next_cursor:string|null}>(`/collection-search?${query}`,{signal:attempt.signal}),{success:page=>setCollectionPage({key:collectionKey,items:page.items,cursor:page.next_cursor,ready:true,loading:false,error:''}),failure:e=>{setCollectionPage(state=>state.key===collectionKey?{...state,error:requestErrorMessage(e)}:state);if(isApiRequestError(e)&&e.code==='unauthenticated')handleFailure(e);},settled:()=>setCollectionPage(state=>state.key===collectionKey?{...state,loading:false}:state)});},150);return()=>clearTimeout(timer);},[collectionKey,refresh]);
 useEffect(()=>{if(!actor)return;const controller=new AbortController();Promise.all([api<{items:Collection[]}>(`/collection-search?limit=1&min_role=1`,{signal:controller.signal}),api<{items:Collection[]}>(`/collection-search?limit=1&min_role=2`,{signal:controller.signal})]).then(([curators,owners])=>{if(!controller.signal.aborted){setCanCurateAny(curators.items.length>0);setCanManageAny(owners.items.length>0);}}).catch(e=>{if(!controller.signal.aborted)handleFailure(e);});return()=>controller.abort();},[actor,refresh]);
 useEffect(()=>{if(!actor||!collection){setSelectedCollectionExact(undefined);return;}const known=collections.find(value=>value.id===collection);if(known){setSelectedCollectionExact(known);return;}const controller=new AbortController();api<Collection>(`/collections/${collection}`,{signal:controller.signal}).then(value=>{if(!controller.signal.aborted)setSelectedCollectionExact(value);}).catch(e=>{if(!controller.signal.aborted)handleFailure(e);});return()=>controller.abort();},[actor,collection,collections]);
 async function moreCollections(){if(!collectionCurrent||!collectionCursor||collectionLoading)return;setCollectionPage(state=>state.key===collectionKey?{...state,loading:true,error:''}:state);const query=new URLSearchParams({search:collectionSearch,limit:'50',cursor:collectionCursor});await collectionReads.run(attempt=>api<{items:Collection[];next_cursor:string|null}>(`/collection-search?${query}`,{signal:attempt.signal}),{success:page=>setCollectionPage(state=>state.key===collectionKey?{...state,items:[...state.items,...page.items.filter(value=>!state.items.some(previous=>previous.id===value.id))],cursor:page.next_cursor}:state),failure:e=>{setCollectionPage(state=>state.key===collectionKey?{...state,error:requestErrorMessage(e)}:state);if(isApiRequestError(e)&&e.code==='unauthenticated')handleFailure(e);},settled:()=>setCollectionPage(state=>state.key===collectionKey?{...state,loading:false}:state)});}
 useEffect(()=>{let active=true;if(actor)api<CollectionType[]>('/collection-types').then(value=>{if(active)setCollectionTypes(value);}).catch(e=>{if(active)handleFailure(e);});return()=>{active=false;};},[actor]);
 useEffect(()=>{
  if(!actor)return;const current=++generation.current;const controller=new AbortController();setBusy(true);setListFailed(false);const scope=JSON.stringify([actor.user_id,collection,search,sort,filter]);if(listScope.current!==scope){setItems([]);setCursor(null);listScope.current=scope;}
  const timer=setTimeout(async()=>{try{
   const previous=pageCache.current;const append=previous?.scope===scope&&previous.refresh===refresh&&previous.pages<navigation.pages;
   let next:string|null=append?previous.cursor:null;let found:Item[]=append?previous.items:[];
   for(let pageIndex=append?previous.pages:0;pageIndex<navigation.pages;pageIndex++){
    const query=new URLSearchParams({limit:'50',sort,filter});if(collection)query.set('collection_id',collection);if(search)query.set('search',search);if(next)query.set('cursor',next);
    const page=await api<Page>(`/items?${query}`,{signal:controller.signal});if(controller.signal.aborted)return;
    found=[...found,...page.items];next=page.next_cursor??null;if(!next)break;
   }
   if(current===generation.current){pageCache.current={scope,refresh,items:found,cursor:next,pages:navigation.pages};setItems(found);setCursor(next);setError('');}
  }catch(e){if(!controller.signal.aborted&&current===generation.current){setItems([]);setCursor(null);setListFailed(true);handleFailure(e);}}
  finally{if(!controller.signal.aborted&&current===generation.current)setBusy(false);}},180);
  return()=>{clearTimeout(timer);controller.abort();generation.current++;};
 },[actor,collection,search,sort,filter,refresh,navigation.pages]);

 useEffect(()=>{
  if(!actor||!route.item){setSelected(undefined);setDetailLoading(false);setDetailError('');return;}
  const controller=new AbortController();setSelected(current=>current?.id===route.item?current:undefined);setDetailLoading(true);setDetailError('');
  api<Item>(`/items/${encodeURIComponent(route.item)}`,{signal:controller.signal}).then(item=>{
   if(controller.signal.aborted)return;
   if(item.deleted&&filter!=='removed')throw new Error('This entry has been removed. Open Removed entries to restore it.');
   if(collection&&item.collection_id!==collection)throw new Error('This entry is outside the selected collection. Return to your results.');
   setSelected(item);
  }).catch(e=>{if(controller.signal.aborted)return;setSelected(undefined);if(isApiRequestError(e)&&e.code==='unauthenticated')handleFailure(e);else setDetailError(isApiRequestError(e)&&[403,404].includes(e.status)?'This entry is unavailable or you no longer have access. Return to your results.':requestErrorMessage(e));}).finally(()=>{if(!controller.signal.aborted)setDetailLoading(false);});
  return()=>controller.abort();
 },[actor,route.item,collection,filter,refresh,detailRetry]);
 useEffect(()=>{if(route.item&&!management)detailsRef.current?.focus({preventScroll:true});},[route.item,detailLoading,management]);
 useEffect(()=>{
  if(busy||management)return;const target=navigation.restore.current;if(!target)return;
  const frame=requestAnimationFrame(()=>{if(!route.item&&target.focus)document.querySelector<HTMLElement>(`[data-item-id="${CSS.escape(target.focus)}"]`)?.focus({preventScroll:true});window.scrollTo(0,target.scroll);navigation.restore.current=null;});
  return()=>cancelAnimationFrame(frame);
 },[busy,items,route.item,navigation.revision,management]);
 function openItem(item:Item){managementRequest.current?.abort();navigate({item:item.id});}
 function loadMore(){if(cursor&&!busy)navigation.setPages(navigation.pages+1);}
 async function changeItemVisibility(item:Item,removed:boolean){
  if(removed&&!window.confirm(`Remove “${item.title}” from browsing? Original files, evidence, artwork, copies and loans remain preserved. Active work for this entry will stop.`))return;
  try{
   await api<Item>(`/items/${item.id}/${removed?'remove':'restore'}`,{method:'POST',body:JSON.stringify({expected_revision:item.revision})});
   setNotice(removed?'Entry removed. Its originals and history remain preserved.':'Entry restored with the same identity and preserved information.');
   navigate({filter:removed?'removed':'all',item:item.id});setRefresh(value=>value+1);
  }catch(error){handleFailure(error,'You no longer have permission to change this entry.');}
 }
 async function openManagement(kind:'curation'|'administration'|'security'|'work-history'|'collection-access',adding=false,initialSection:'details'|'files'|'evidence'|'reimport'='details'){
  if(kind==='work-history'&&!new URL(location.href).searchParams.has('work')){const url=new URL(location.href);url.searchParams.set('work','list');history.pushState({...history.state},'',url);}
  managementRequest.current?.abort();const controller=new AbortController();managementRequest.current=controller;
  try{
   // Browse rows can lag a just-published Capture. Open curation from a fresh
   // authorized Item so its module panels never inherit an obsolete graph.
   const [module,currentItem]=await Promise.all([
    import(/* @vite-ignore */ `/${kind}.js`),
    kind==='curation'&&!adding&&selected?api<Item>(`/items/${selected.id}`,{signal:controller.signal}):Promise.resolve(undefined),
   ]);
   if(controller.signal.aborted)return;
   if(currentItem)setSelected(currentItem);
   const targetCollection=currentItem?.collection_id||selected?.collection_id||collection;
   const exact=targetCollection&&!collections.some(value=>value.id===targetCollection)?await api<Collection>(`/collections/${targetCollection}`,{signal:controller.signal}):undefined;
   if(controller.signal.aborted)return;
   const available=exact?[exact,...collections]:collections;
   setManagement(React.createElement(module.default,{collections:kind==='curation'?available.filter(value=>value.content_permissions.can_edit):available,initialCollectionId:kind==='collection-access'?targetCollection:available.find(value=>value.id===targetCollection&&value.content_permissions.can_edit)?.id,collectionTypes,initialSection,selected:currentItem,initialItemId:kind==='work-history'?route.item:undefined,onClose:()=>{controller.abort();if(kind==='work-history'){const url=new URL(location.href);url.searchParams.delete('work');history.replaceState({...history.state},'',url);}setManagement(null);setRefresh(n=>n+1);},onSaved:(item:Item,replacedItemId?:string)=>{if(controller.signal.aborted)return;if(kind!=='work-history'&&(!route.item||route.item===item.id||route.item===replacedItemId)){navigate({item:item.id},!!route.item);setSelected(item);}setRefresh(n=>n+1);},onSessionExpired:()=>{if(!controller.signal.aborted)clearSession('Your session expired or was revoked. Sign in again; the failed operation was not saved.');}}));
  }catch(e){if(!controller.signal.aborted)handleFailure(e);}
 }
 useEffect(()=>{const open=()=>void openManagement('work-history');window.addEventListener('collector-open-work',open);return()=>window.removeEventListener('collector-open-work',open);},[collections,route.item,collection]);
 const openedWorkLink=useRef(false);
 useEffect(()=>{if(actor&&collectionsReady&&features?.curation&&new URL(location.href).searchParams.has('work')&&!openedWorkLink.current){openedWorkLink.current=true;void openManagement('work-history');}},[actor,collectionsReady,features?.curation]);
 useEffect(()=>{const reopen=()=>{if(actor&&collectionsReady&&features?.curation&&!management&&new URL(location.href).searchParams.has('work'))void openManagement('work-history');};window.addEventListener('popstate',reopen);return()=>window.removeEventListener('popstate',reopen);},[actor,collectionsReady,features?.curation,management,collections,route.item,collection]);
 async function signOut(){
  try{await api('/auth/logout',{method:'POST'});clearSession('');}
  catch(e){if(isApiRequestError(e)&&e.code==='unauthenticated')clearSession('');else handleFailure(e);}
 }

 if(!ready)return <div className="loading">Opening your collection…</div>;
 if(!actor)return <Login features={features} error={loginMessage||error} onLogin={value=>{setActor(value);setError('');setLoginMessage('');}}/>;
 const clientOnly=!!features&&!features.curation&&!features.administration;
 const noCollections=collectionsReady&&collections.length===0&&!collectionSearch;
 const selectedCollection=selectedCollectionExact?.id===collection?selectedCollectionExact:collections.find(c=>c.id===collection);
 return <div className={`shell ${route.item?'viewing-entry':''}`}><aside className="sidebar"><a className="brand" href="/">Junk<br/>Collector<span className="brand-mark">✳</span></a><p className="sidebar-note">Keep the collection.<br/>Keep the evidence.</p><nav aria-label="Collections"><label className="sidebar-search">Find a collection<input type="search" maxLength={256} value={collectionSearch} onChange={event=>setCollectionSearch(event.target.value)}/></label><button className={!collection?'active':''} onClick={()=>changeScope({collection:''})}>All collections</button>{selectedCollection&&!collections.some(value=>value.id===selectedCollection.id)&&<button className="active" onClick={()=>changeScope({collection:selectedCollection.id})}>{selectedCollection.name}</button>}{collections.map(c=><button key={c.id} className={collection===c.id?'active':''} onClick={()=>changeScope({collection:c.id})}>{c.name}</button>)}{collectionLoading&&<span role="status">Loading collections…</span>}{collectionError&&<span role="alert">{collectionError} <button onClick={()=>setCollectionRevision(value=>value+1)}>Retry</button></span>}{collectionCursor&&<button disabled={collectionLoading} onClick={()=>void moreCollections()}>More collections</button>}</nav><div className="sidebar-bottom">{features?.passkeys&&<button onClick={async()=>{try{await registerPasskey();setNotice('Passkey registered. You can use it on the next sign-in.');}catch(e){handleFailure(e);}}}>Add a passkey</button>}{features?.curation&&<button onClick={()=>openManagement('work-history')}>Work &amp; history</button>}<button onClick={()=>openManagement('security')}>Account &amp; security</button>{features?.curation&&canManageAny&&<button onClick={()=>openManagement('collection-access')}>Manage collection access</button>}{features?.curation&&canCurateAny&&<button onClick={()=>openManagement('curation')}>Manage collections</button>}{features?.administration&&actor.administrator&&<button onClick={()=>openManagement('administration')}>Administration</button>}<button onClick={signOut}>Sign out</button></div></aside>
 <main><header className="page-heading"><div><p className="eyebrow">Your archive</p><h1>{selectedCollection?.name||'All collections'}</h1></div><span className="collection-count">{items.length}{cursor?'+':''} {items.length===1&&!cursor?'entry':'entries'}</span></header>
 {clientOnly&&<div className="notice" role="note"><strong>Client access only at this address.</strong> You can browse and play permitted media here. To create collections or change inventory, open the private management address printed by the server. Never publish that private listener through a public tunnel.</div>}
 {selectedCollection?.content_permissions.message&&<ContentNotice collection={selectedCollection}/>}<div className="toolbar"><label className="search"><span aria-hidden="true">⌕</span><input type="search" value={search} maxLength={256} onChange={e=>changeScope({search:e.target.value},true)} placeholder="Find a title, artist, or platform" aria-label="Search collection"/></label>{features?.curation&&canCurateAny&&(!collection||selectedCollection?.content_permissions.can_edit)&&<button className="primary" disabled={!collectionTypes.length} onClick={()=>openManagement('curation',true)}>Add / import</button>}</div>
 <div className="browse-options"><label>Sort by<select aria-label="Sort by" value={sort} onChange={e=>changeScope({sort:e.target.value as BrowseLocation['sort']})}><option value="title">Title</option><option value="recently_added">Recently added</option><option value="artist">Artist</option><option value="platform">Platform</option></select></label><label>Show<select aria-label="Show" value={filter} onChange={e=>changeScope({filter:e.target.value as BrowseLocation['filter']})}><option value="all">All entries</option><option value="wishlist">Wishlist</option><option value="files_attached">Files attached</option><option value="playable">Playable audio</option><option value="needs_attention">Needs my attention</option>{features?.curation&&canCurateAny&&<option value="removed">Removed entries</option>}</select></label>{(search||filter!=='all')&&<button onClick={()=>changeScope({search:'',filter:'all'})}>Clear filters</button>}</div>
 {filter==='playable'&&<p className="muted">Published tracks or audio files are available. Playback support depends on your browser.</p>}{filter==='needs_attention'&&<p className="muted">Your failed jobs and imports awaiting review. Open an entry to inspect its work.</p>}{filter==='removed'&&<p className="muted">Removed entries are hidden from ordinary browsing. Originals, evidence, artwork, copies and loans remain preserved. Open an entry to restore it.</p>}
 {notice&&<div role="status" className="notice">{notice}<button onClick={()=>setNotice('')} aria-label="Dismiss notice">×</button></div>}{error&&<div role="alert" className="notice error">{error}<button onClick={()=>setError('')} aria-label="Dismiss error">×</button></div>}
 <div className={`browser ${route.item?'has-detail':''}`}><section className="entry-list" aria-label="Collection entries" aria-busy={busy}><div className="list-heading"><span>Title / edition</span><span>Identity</span></div>{!items.length&&!busy&&!listFailed?<EmptyState search={search||(filter!=='all'?'filtered':'')} noCollections={noCollections} canAdminister={!!features?.administration&&actor.administrator} canCurate={canCurateAny} onCreate={()=>openManagement('administration')}/>:items.map(item=>{const type=collectionTypes.find(value=>value.id===item.domain.module);const summary=item.presentation.fields.filter(value=>value.label!=='Collection type').slice(0,2).map(value=>value.value).join(' · ');return <button className={`collection-row ${selected?.id===item.id?'selected':''}`} key={item.id} data-item-id={item.id} aria-expanded={route.item===item.id} onClick={()=>openItem(item)}><Artwork item={item}/><span className="row-text"><strong>{item.title}</strong><span>{[item.subtitle,summary].filter(Boolean).join(' · ')||'Edition details not yet added'}</span><small>{type?.collection_type_label||`Unavailable type · ${item.domain.module}`}{item.wishlist?' · Wishlist':''}</small></span><span className={`identity ${item.identity}`}>{item.identity==='tentative'?'Tentative match':item.identity==='unidentified'?'Unidentified':item.identity==='confirmed'?'Confirmed identity':'Selected identity'}</span></button>;})}{listFailed&&<div className="empty"><h2>Entries could not be loaded</h2><p>Your collection has not been changed.</p><button onClick={()=>setRefresh(v=>v+1)}>Retry loading entries</button></div>}{cursor&&<button className="load-more" disabled={busy} onClick={loadMore}>Load more</button>}{busy&&<p role="status" className="muted">Loading entries…</p>}</section>
 {route.item&&<section ref={detailsRef} tabIndex={-1} className="detail-view" aria-label="Selected entry"><button className="back-to-results" onClick={()=>{managementRequest.current?.abort();closeDetails();}}>Back to results</button>{detailLoading&&<p role="status">{selected?.id===route.item?'Updating entry…':'Loading entry…'}</p>}{detailError&&<div role="alert" className="notice error">{detailError}<button onClick={()=>setDetailRetry(v=>v+1)}>Retry entry</button></div>}{selected?.id===route.item&&<Detail key={selected.id} refresh={refresh} item={selected} curation={!!features?.curation&&selected.content_permissions.can_edit} accessManagement={!!features?.curation&&selected.effective_permissions.can_manage_access} onEdit={()=>openManagement('curation')} onAttach={()=>openManagement('curation',false,'files')} onReimport={()=>openManagement('curation',false,'reimport')} onRemove={()=>void changeItemVisibility(selected,true)} onRestore={()=>void changeItemVisibility(selected,false)} onManageAccess={()=>openManagement('collection-access')} onOpenHistory={()=>openManagement('work-history')} onClose={()=>{managementRequest.current?.abort();closeDetails();}} onSessionExpired={()=>clearSession('Your session expired or was revoked. Sign in again.')}/>}</section>}</div></main><MusicPlayer key={actor.user_id} browsingMusic={selectedCollection?.binding.state==='assigned'&&selectedCollection.binding.module==='org.junk.music'||selected?.domain.module==='org.junk.music'||filter==='playable'} onSessionExpired={()=>clearSession('Your session expired or was revoked. Sign in again.')}/>{management}{reauth!==null&&<RecentAuthentication key={reauth} username={reauthUsername} passkeys={!!features?.passkeys} onAuthenticated={value=>finishReauth(value,reauth)} onCancel={()=>finishReauth(undefined,reauth)}/>}</div>;
}

function ContentNotice({collection}:{collection:Collection}){
 return <div className="notice" role="note"><strong>{collection.content_permissions.status==='legacy_unassigned'?'Legacy collection type not assigned.':'Collection content is read-only.'}</strong> {collection.content_permissions.message}{collection.content_permissions.administrator_commands?.map(command=><code className="admin-command" key={command}>{command}</code>)}</div>;
}

function EmptyState({search,noCollections,canAdminister,canCurate,onCreate}:{search:string;noCollections:boolean;canAdminister:boolean;canCurate:boolean;onCreate:()=>void}){
 if(search)return <div className="empty"><span aria-hidden="true">▤</span><h2>No matching entries</h2><p>Try another search or clear the filters.</p></div>;
 if(noCollections&&canAdminister)return <div className="empty"><span aria-hidden="true">▤</span><h2>Start with a collection.</h2><p>Create a collection here, then add games or CDs. Original files and artwork can be attached after the inventory entry is saved.</p><button className="primary" onClick={onCreate}>Create collection</button></div>;
 if(noCollections)return <div className="empty"><span aria-hidden="true">▤</span><h2>No collections are visible.</h2><p>Ask a collection owner to grant this profile access. Creating a collection requires an administrator using the private management address printed by the server.</p></div>;
 if(canCurate)return <div className="empty"><span aria-hidden="true">▤</span><h2>A place for every edition.</h2><p>This collection has no entries yet. Use Add / import to preserve a package or add a game or CD manually.</p></div>;
 return <div className="empty"><span aria-hidden="true">▤</span><h2>This collection is empty.</h2><p>A curator can add its first game or CD from a management-enabled address.</p></div>;
}

function RecentAuthentication({username,passkeys,onAuthenticated,onCancel}:{username:string;passkeys:boolean;onAuthenticated:(principal:Principal)=>void;onCancel:()=>void}){
 const [password,setPassword]=useState('');const [error,setError]=useState('');const [busy,setBusy]=useState(false);const panel=useRef<HTMLElement>(null);const operation=useOperationScope(username);
 function cancel(){operation.cancel();onCancel();}
 useEffect(()=>{const previous=document.activeElement as HTMLElement|null;function keydown(event:KeyboardEvent){if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();cancel();}if(event.key==='Tab'&&panel.current){const controls=[...panel.current.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled)')];const first=controls[0],last=controls.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus();}}}document.addEventListener('keydown',keydown,true);return()=>{operation.cancel();document.removeEventListener('keydown',keydown,true);if(previous?.isConnected)previous.focus();};},[]);
 async function authenticate(method:'password'|'passkey'){
  setBusy(true);setError('');await operation.run(async attempt=>{
   if(method==='passkey')await usePasskey(username,attempt.signal);
   else await api('/auth/login',{method:'POST',signal:attempt.signal,body:JSON.stringify({username,password,device_name:'Web browser'})});
   return api<Principal>('/me',{signal:attempt.signal});
  },{success:value=>{setPassword('');onAuthenticated(value);},failure:value=>setError(requestErrorMessage(value)),settled:()=>setBusy(false)});
 }

 return <div className="modal-backdrop reauth-backdrop"><section ref={panel} className="modal" role="dialog" aria-modal="true" aria-labelledby="reauth-title"><h2 id="reauth-title">Confirm it is you</h2><p>Your draft is still here. Reauthenticate as {username}; the pending action will be submitted after success. Cancel to review and submit it yourself later.</p><form onSubmit={event=>{event.preventDefault();void authenticate('password');}}><label>Password<input autoFocus type="password" autoComplete="current-password" value={password} onChange={event=>setPassword(event.target.value)}/></label><button disabled={busy||!password}>Continue</button></form>{passkeys&&<button disabled={busy} onClick={()=>void authenticate('passkey')}>Use a passkey</button>}<button onClick={cancel}>Cancel and keep draft</button>{error&&<p role="alert" className="notice error">{error}</p>}</section></div>;
}

function Login({features,error,onLogin}:{features?:Features;error:string;onLogin:(actor:Principal)=>void}){
 const [username,setUsername]=useState('owner');const [password,setPassword]=useState('');const [code,setCode]=useState('');const [newPassword,setNewPassword]=useState('');const [recovering,setRecovering]=useState(false);const [message,setMessage]=useState(error);const [busy,setBusy]=useState(false);
 useEffect(()=>{if(!features?.multi_user&&features?.default_username)setUsername(features.default_username);},[features]);
 useEffect(()=>{setMessage(error);},[error]);
 async function completeLogin(){const principal=await api<Principal>('/me');setPassword('');setCode('');setNewPassword('');onLogin(principal);}
 async function submitPassword(){setBusy(true);setMessage('');try{await api('/auth/login',{method:'POST',body:JSON.stringify({username,password,device_name:'Web browser'})});await completeLogin();}catch(e){setMessage(requestErrorMessage(e));}finally{setBusy(false);}}
 async function submitRecovery(){setBusy(true);setMessage('');try{await api('/auth/recovery',{method:'POST',body:JSON.stringify({username,code,new_password:newPassword,device_name:'Web browser recovery'})});await completeLogin();}catch(e){setMessage(requestErrorMessage(e));}finally{setBusy(false);}}
 return <main className="login"><div className="login-intro"><p>Junk Collector</p><h1>Your collection,<br/>with its story intact.</h1><p>Games, records, original files, and the evidence that connects them.</p></div><form onSubmit={event=>{event.preventDefault();void (recovering?submitRecovery():submitPassword());}}><h2>{recovering?'Recover your account':'Open your collection'}</h2><p>{recovering?'Use one unused recovery code to set a new password. Recovery replaces existing browser sessions and invalidates the old codes.':<>Use the password created by the server’s setup command. {features&&!features.administration&&'This is a client-access address; use the private management address printed by the server for initial setup and inventory changes.'}</>}</p>{features?.multi_user?<label>Username<input autoComplete="username" value={username} onChange={e=>setUsername(e.target.value)}/></label>:<input type="hidden" autoComplete="username" value={username}/>} {recovering?<><label>Recovery code<input required autoComplete="off" value={code} onChange={e=>setCode(e.target.value)}/></label><label>New password<input required minLength={12} type="password" autoComplete="new-password" value={newPassword} onChange={e=>setNewPassword(e.target.value)}/></label></>:<label>Password<input required type="password" autoComplete="current-password" value={password} onChange={e=>setPassword(e.target.value)}/></label>}{message&&<p role="alert" className="error">{message}</p>}<button className="primary" disabled={busy}>{busy?'Working…':recovering?'Use recovery code':'Sign in'}</button>{!recovering&&features?.passkeys&&<button type="button" disabled={busy} onClick={async()=>{setBusy(true);setMessage('');try{await usePasskey(username);await completeLogin();}catch(e){setMessage(requestErrorMessage(e));}finally{setBusy(false);}}}>Sign in with a passkey</button>}<button type="button" disabled={busy} onClick={()=>{setRecovering(value=>!value);setMessage('');setPassword('');setCode('');setNewPassword('');}}>{recovering?'Back to sign in':'Use a recovery code'}</button></form></main>;
}

function Detail({item,refresh,curation,accessManagement,onEdit,onAttach,onReimport,onRemove,onRestore,onManageAccess,onOpenHistory,onClose,onSessionExpired}:{item:Item;refresh:number;curation:boolean;accessManagement:boolean;onEdit:()=>void;onAttach:()=>void;onReimport:()=>void;onRemove:()=>void;onRestore:()=>void;onManageAccess:()=>void;onOpenHistory:()=>void;onClose:()=>void;onSessionExpired:()=>void}){
 const [evidence,setEvidence]=useState<{id:string;kind:string;outcome:string}[]>([]);const [error,setError]=useState('');const [loading,setLoading]=useState(true);const [retry,setRetry]=useState(0);
 useEffect(()=>{const controller=new AbortController();setEvidence([]);setError('');setLoading(true);api<typeof evidence>(`/items/${item.id}/evidence`,{signal:controller.signal}).then(value=>{if(!controller.signal.aborted)setEvidence(value);}).catch(e=>{if(controller.signal.aborted)return;if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}).finally(()=>{if(!controller.signal.aborted)setLoading(false);});return()=>controller.abort();},[item.id,item.revision,refresh,retry]);
 const [queueNotice,setQueueNotice]=useState('');
 const presentation=item.presentation;
 return <aside className="detail" aria-label={`${item.title} details`}><button className="close" onClick={onClose} aria-label="Close details">×</button><Artwork item={item} large/><h2>{item.title}</h2><p>{item.subtitle}</p>{curation&&<div className="item-lifecycle">{item.deleted?<button onClick={onRestore}>Restore entry</button>:<button onClick={onRemove}>Remove item</button>}</div>}{item.deleted&&<p className="notice">This entry is removed from ordinary browsing. Its original files, evidence, artwork, copies and loans remain preserved. Restore it to download or play stored files.</p>}<dl><dt>Physical copies</dt><dd>{item.specimens.length}</dd><dt>Captures</dt><dd>{item.captures.length}</dd><dt>Identity</dt><dd>{item.identity}</dd></dl>{curation&&!item.deleted?<><button onClick={onEdit}>Edit / attach files</button><button onClick={onAttach}>Attach original files</button>{item.captures.length>0&&<button onClick={onReimport}>Replace original files</button>}</>:accessManagement&&!item.deleted&&<button onClick={onManageAccess}>Manage collection access</button>}{!item.deleted&&<MusicTracks key={item.id} item={item} canPrepare={false} onOpenHistory={onOpenHistory} onSessionExpired={onSessionExpired}/>}{presentation&&<section aria-label="Module details"><h3>Module details</h3>{presentation.fields.map(field=><p key={field.label}><strong>{field.label}:</strong> {field.value}</p>)}{presentation.badges.map(badge=><span className={`identity ${badge.tone}`} key={badge.label}>{badge.label}</span>)}{presentation.limitations.map(value=><p className="muted" key={value}>{value}</p>)}{!presentation.available&&presentation.values.map(value=><details key={value.label}><summary>{value.label}</summary><pre>{JSON.stringify(value.value,null,2)}</pre></details>)}</section>}<h3>Evidence</h3>{loading?<p role="status">Loading evidence…</p>:error?<div role="alert" className="error">Evidence could not be loaded. {error}<button onClick={()=>setRetry(v=>v+1)}>Retry evidence</button></div>:!evidence.length?<p className="muted">No evidence recorded.</p>:evidence.map(e=><p className="evidence" key={e.id}><span>{e.kind.replaceAll('_',' ')}</span><strong>{e.outcome}</strong></p>)}<h3>Stored files</h3>{queueNotice&&<p role="status">{queueNotice}</p>}{item.captures.flatMap(c=>c.representations).flatMap(r=>r.files.map((f,i)=>{const url=`/api/v1/items/${item.id}/representations/${r.id}/${i}`;return <div className="stored-file" key={`${r.id}/${i}`}>{item.deleted?<span>{f.name}</span>:<a href={url} download={f.name}>{f.name}</a>}<small>{(f.bytes/1048576).toFixed(1)} MiB · {r.role==='preservation_master'?'Original':r.role==='lossless_derivative'?'Lossless copy':'Lossy copy'}</small>{!item.deleted&&r.role==='preservation_master'&&audioMediaType(f.name)&&<button onClick={()=>{const count=addMusicTracks([{itemId:item.id,collectionId:item.collection_id,album:item.title,title:f.name,position:i+1,representationId:r.id,fileIndex:i,mediaType:audioMediaType(f.name)!,artwork:item.primary_artwork||undefined,original:true}]);setQueueNotice(count?'Added file to your queue.':'Your queue is full. Remove tracks to add more.');}}>Add file to queue</button>}</div>;}))}</aside>;
}

createRoot(document.getElementById('root')!).render(<App/>);
