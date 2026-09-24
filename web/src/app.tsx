import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from './runtime';
import {createRoot} from 'react-dom/client';
import type {components} from './generated/api';
import MusicTracks from './music-tracks';
import MusicPlayer from './music-player';
import {setMusicQueueUser,addMusicTracks,audioMediaType} from './music-queue';
import {useBrowseNavigation,type BrowseLocation} from './browse-navigation';

export {useMusicQueue,addMusicTracks} from './runtime';
export {default,useState,useEffect,useRef,ApiRequestError,isApiRequestError,requestErrorMessage,api} from './runtime';

type Collection=components['schemas']['PresentedCollection'];
type Features=components['schemas']['FeaturesResponse'];
type Item=components['schemas']['PresentedItem'];
type Page=components['schemas']['Page_PresentedItem'];
type Principal=components['schemas']['Principal'];
type CollectionType=components['schemas']['ModuleDescriptor'];

async function usePasskey(username:string):Promise<Principal>{
 const started=await api<{ceremony_id:string;options:{publicKey:PublicKeyCredentialRequestOptionsJSON}}>('/auth/passkeys/login/start',{method:'POST',body:JSON.stringify({username})});
 const publicKey=PublicKeyCredential.parseRequestOptionsFromJSON(started.options.publicKey);
 const credential=await navigator.credentials.get({publicKey}) as PublicKeyCredential|null;
 if(!credential)throw new Error('Passkey sign-in was cancelled.');
 const result=await api<{principal:Principal}>('/auth/passkeys/login/finish',{method:'POST',body:JSON.stringify({ceremony_id:started.ceremony_id,credential:credential.toJSON()})});return result.principal;
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
 const [features,setFeatures]=useState<Features>();const [actor,setActor]=useState<Principal>();const [ready,setReady]=useState(false);
 const [collections,setCollections]=useState<Collection[]>([]);const [collectionTypes,setCollectionTypes]=useState<CollectionType[]>([]);const [items,setItems]=useState<Item[]>([]);const [cursor,setCursor]=useState<string|null>(null);const [selected,setSelected]=useState<Item>();const [error,setError]=useState('');const [loginMessage,setLoginMessage]=useState('');const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false);const [refresh,setRefresh]=useState(0);
 useEffect(()=>{function escape(event:KeyboardEvent){if(event.key==='Escape'&&!document.querySelector('[role="dialog"]')&&route.item){managementRequest.current?.abort();closeDetails();}}document.addEventListener('keydown',escape);return()=>document.removeEventListener('keydown',escape);},[route.item]);
 useEffect(()=>{setMusicQueueUser(actor?.user_id);return()=>setMusicQueueUser(undefined);},[actor?.user_id]);
 const [management,setManagement]=useState<React.ReactNode>(null);const generation=useRef(0);const managementRequest=useRef<AbortController|null>(null);

 function clearSession(message:string){
  setMusicQueueUser(undefined);
  managementRequest.current?.abort();generation.current++;pageCache.current=undefined;setBusy(false);setItems([]);setCollections([]);setCollectionTypes([]);setSelected(undefined);setManagement(null);setError('');setNotice('');setLoginMessage(message);setActor(undefined);
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
 useEffect(()=>{let active=true;if(actor)api<Collection[]>('/collections').then(value=>{if(active){setCollections(value);}}).catch(e=>{if(active)handleFailure(e);});return()=>{active=false;};},[actor,refresh]);
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
   if(item.deleted)throw new Error('This entry has been removed. Return to your results.');
   if(collection&&item.collection_id!==collection)throw new Error('This entry is outside the selected collection. Return to your results.');
   setSelected(item);
  }).catch(e=>{if(controller.signal.aborted)return;setSelected(undefined);if(isApiRequestError(e)&&e.code==='unauthenticated')handleFailure(e);else setDetailError(isApiRequestError(e)&&[403,404].includes(e.status)?'This entry is unavailable or you no longer have access. Return to your results.':requestErrorMessage(e));}).finally(()=>{if(!controller.signal.aborted)setDetailLoading(false);});
  return()=>controller.abort();
 },[actor,route.item,collection,refresh,detailRetry]);
 useEffect(()=>{if(route.item&&!management)detailsRef.current?.focus({preventScroll:true});},[route.item,detailLoading,management]);
 useEffect(()=>{
  if(busy||management)return;const target=navigation.restore.current;if(!target)return;
  const frame=requestAnimationFrame(()=>{if(!route.item&&target.focus)document.querySelector<HTMLElement>(`[data-item-id="${CSS.escape(target.focus)}"]`)?.focus({preventScroll:true});window.scrollTo(0,target.scroll);navigation.restore.current=null;});
  return()=>cancelAnimationFrame(frame);
 },[busy,items,route.item,navigation.revision,management]);
 function openItem(item:Item){managementRequest.current?.abort();navigate({item:item.id});}
 function loadMore(){if(cursor&&!busy)navigation.setPages(navigation.pages+1);}
 async function openManagement(kind:'curation'|'administration'|'security'|'work-history',adding=false){
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
   setManagement(React.createElement(module.default,{collections:kind==='curation'?collections.filter(value=>value.content_permissions.can_edit||(!adding&&value.effective_permissions.can_manage_access)):collections,collectionTypes,initialCollectionId:collections.find(value=>value.id===collection&&value.content_permissions.can_edit)?.id,selected:currentItem,initialItemId:kind==='work-history'?route.item:undefined,onClose:()=>{controller.abort();setManagement(null);setRefresh(n=>n+1);},onSaved:(item:Item,replacedItemId?:string)=>{if(controller.signal.aborted)return;if(!route.item||route.item===item.id||route.item===replacedItemId){navigate({item:item.id},!!route.item);setSelected(item);}setRefresh(n=>n+1);},onSessionExpired:()=>{if(!controller.signal.aborted)clearSession('Your session expired or was revoked. Sign in again; the failed operation was not saved.');}}));
  }catch(e){if(!controller.signal.aborted)handleFailure(e);}
 }
 useEffect(()=>{const open=()=>void openManagement('work-history');window.addEventListener('collector-open-work',open);return()=>window.removeEventListener('collector-open-work',open);},[collections,route.item,collection]);
 async function signOut(){
  try{await api('/auth/logout',{method:'POST'});clearSession('');}
  catch(e){if(isApiRequestError(e)&&e.code==='unauthenticated')clearSession('');else handleFailure(e);}
 }

 if(!ready)return <div className="loading">Opening your collection…</div>;
 if(!actor)return <Login features={features} error={loginMessage||error} onLogin={value=>{setActor(value);setError('');setLoginMessage('');}}/>;
 const clientOnly=!!features&&!features.curation&&!features.administration;
 const noCollections=collections.length===0;
 const selectedCollection=collections.find(c=>c.id===collection);
 return <div className={`shell ${route.item?'viewing-entry':''}`}><aside className="sidebar"><a className="brand" href="/">Junk<br/>Collector<span className="brand-mark">✳</span></a><p className="sidebar-note">Keep the collection.<br/>Keep the evidence.</p><nav aria-label="Collections"><button className={!collection?'active':''} onClick={()=>changeScope({collection:''})}>All collections</button>{collections.map(c=><button key={c.id} className={collection===c.id?'active':''} onClick={()=>changeScope({collection:c.id})}>{c.name}</button>)}</nav><div className="sidebar-bottom">{features?.passkeys&&<button onClick={async()=>{try{await registerPasskey();setNotice('Passkey registered. You can use it on the next sign-in.');}catch(e){handleFailure(e);}}}>Add a passkey</button>}{features?.curation&&<button onClick={()=>openManagement('work-history')}>Work &amp; history</button>}<button onClick={()=>openManagement('security')}>Account &amp; security</button>{features?.curation&&collections.some(value=>value.content_permissions.can_edit||value.effective_permissions.can_manage_access)&&<button onClick={()=>openManagement('curation')}>Manage collections</button>}{features?.administration&&actor.administrator&&<button onClick={()=>openManagement('administration')}>Administration</button>}<button onClick={signOut}>Sign out</button></div></aside>
 <main><header className="page-heading"><div><p className="eyebrow">Your archive</p><h1>{selectedCollection?.name||'All collections'}</h1></div><span className="collection-count">{items.length}{cursor?'+':''} {items.length===1&&!cursor?'entry':'entries'}</span></header>
 {clientOnly&&<div className="notice" role="note"><strong>Client access only at this address.</strong> You can browse and play permitted media here. To create collections or change inventory, open the private management address printed by the server. Never publish that private listener through a public tunnel.</div>}
 {selectedCollection?.content_permissions.message&&<ContentNotice collection={selectedCollection}/>}<div className="toolbar"><label className="search"><span aria-hidden="true">⌕</span><input type="search" value={search} maxLength={256} onChange={e=>changeScope({search:e.target.value},true)} placeholder="Find a title, artist, or platform" aria-label="Search collection"/></label>{features?.curation&&collections.some(value=>value.content_permissions.can_edit)&&<button className="primary" disabled={!collectionTypes.length} onClick={()=>openManagement('curation',true)}>Add / import</button>}</div>
 <div className="browse-options"><label>Sort by<select aria-label="Sort by" value={sort} onChange={e=>changeScope({sort:e.target.value as BrowseLocation['sort']})}><option value="title">Title</option><option value="recently_added">Recently added</option><option value="artist">Artist</option><option value="platform">Platform</option></select></label><label>Show<select aria-label="Show" value={filter} onChange={e=>changeScope({filter:e.target.value as BrowseLocation['filter']})}><option value="all">All entries</option><option value="wishlist">Wishlist</option><option value="files_attached">Files attached</option><option value="playable">Playable audio</option><option value="needs_attention">Needs my attention</option></select></label>{(search||filter!=='all')&&<button onClick={()=>changeScope({search:'',filter:'all'})}>Clear filters</button>}</div>
 {filter==='playable'&&<p className="muted">Published tracks or audio files are available. Playback support depends on your browser.</p>}{filter==='needs_attention'&&<p className="muted">Your failed jobs and imports awaiting review. Open an entry to inspect its work.</p>}
 {notice&&<div role="status" className="notice">{notice}<button onClick={()=>setNotice('')} aria-label="Dismiss notice">×</button></div>}{error&&<div role="alert" className="notice error">{error}<button onClick={()=>setError('')} aria-label="Dismiss error">×</button></div>}
 <div className={`browser ${route.item?'has-detail':''}`}><section className="entry-list" aria-label="Collection entries" aria-busy={busy}><div className="list-heading"><span>Title / edition</span><span>Identity</span></div>{!items.length&&!busy&&!listFailed?<EmptyState search={search||(filter!=='all'?'filtered':'')} noCollections={noCollections} canAdminister={!!features?.administration&&actor.administrator} canCurate={collections.some(value=>value.content_permissions.can_edit)} onCreate={()=>openManagement('administration')}/>:items.map(item=>{const type=collectionTypes.find(value=>value.id===item.domain.module);const summary=item.presentation.fields.filter(value=>value.label!=='Collection type').slice(0,2).map(value=>value.value).join(' · ');return <button className={`collection-row ${selected?.id===item.id?'selected':''}`} key={item.id} data-item-id={item.id} aria-expanded={route.item===item.id} onClick={()=>openItem(item)}><Artwork item={item}/><span className="row-text"><strong>{item.title}</strong><span>{[item.subtitle,summary].filter(Boolean).join(' · ')||'Edition details not yet added'}</span><small>{type?.collection_type_label||`Unavailable type · ${item.domain.module}`}{item.wishlist?' · Wishlist':''}</small></span><span className={`identity ${item.identity}`}>{item.identity==='tentative'?'Tentative match':item.identity==='unidentified'?'Unidentified':item.identity==='confirmed'?'Confirmed identity':'Selected identity'}</span></button>;})}{listFailed&&<div className="empty"><h2>Entries could not be loaded</h2><p>Your collection has not been changed.</p><button onClick={()=>setRefresh(v=>v+1)}>Retry loading entries</button></div>}{cursor&&<button className="load-more" disabled={busy} onClick={loadMore}>Load more</button>}{busy&&<p role="status" className="muted">Loading entries…</p>}</section>
 {route.item&&<section ref={detailsRef} tabIndex={-1} className="detail-view" aria-label="Selected entry"><button className="back-to-results" onClick={()=>{managementRequest.current?.abort();closeDetails();}}>Back to results</button>{detailLoading&&<p role="status">{selected?.id===route.item?'Updating entry…':'Loading entry…'}</p>}{detailError&&<div role="alert" className="notice error">{detailError}<button onClick={()=>setDetailRetry(v=>v+1)}>Retry entry</button></div>}{selected?.id===route.item&&<Detail key={selected.id} refresh={refresh} item={selected} curation={!!features?.curation&&selected.content_permissions.can_edit} accessManagement={!!features?.curation&&!!collections.find(value=>value.id===selected.collection_id)?.effective_permissions.can_manage_access} onEdit={()=>openManagement('curation')} onClose={()=>{managementRequest.current?.abort();closeDetails();}} onSessionExpired={()=>clearSession('Your session expired or was revoked. Sign in again.')}/>}</section>}</div></main><MusicPlayer key={actor.user_id} browsingMusic={selectedCollection?.binding.state==='assigned'&&selectedCollection.binding.module==='org.junk.music'||selected?.domain.module==='org.junk.music'||filter==='playable'} onSessionExpired={()=>clearSession('Your session expired or was revoked. Sign in again.')}/>{management}</div>;
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

function Login({features,error,onLogin}:{features?:Features;error:string;onLogin:(actor:Principal)=>void}){
 const [username,setUsername]=useState('owner');const [password,setPassword]=useState('');const [message,setMessage]=useState(error);const [busy,setBusy]=useState(false);
 useEffect(()=>{if(!features?.multi_user&&features?.default_username)setUsername(features.default_username);},[features]);
 useEffect(()=>{setMessage(error);},[error]);
 return <main className="login"><div className="login-intro"><p>Junk Collector</p><h1>Your collection,<br/>with its story intact.</h1><p>Games, records, original files, and the evidence that connects them.</p></div><form onSubmit={async e=>{e.preventDefault();setBusy(true);setMessage('');try{await api<{principal:Principal}>('/auth/login',{method:'POST',body:JSON.stringify({username,password,device_name:'Web browser'})});setPassword('');try{onLogin(await api<Principal>('/me'));}catch{throw new Error('Your browser did not retain the session. Use the exact localhost URL printed by setup, or configure browser-trusted HTTPS.');}}catch(e){setMessage(requestErrorMessage(e));}finally{setBusy(false);}}}><h2>Open your collection</h2><p>Use the password created by the server’s setup command. {features&&!features.administration&&'This is a client-access address; use the private management address printed by the server for initial setup and inventory changes.'}</p>{features?.multi_user?<label>Username<input autoComplete="username" value={username} onChange={e=>setUsername(e.target.value)}/></label>:<input type="hidden" autoComplete="username" value={username}/>}<label>Password<input required type="password" autoComplete="current-password" value={password} onChange={e=>setPassword(e.target.value)}/></label>{message&&<p role="alert" className="error">{message}</p>}<button className="primary" disabled={busy}>{busy?'Signing in…':'Sign in'}</button>{features?.passkeys&&<button type="button" disabled={busy} onClick={async()=>{setBusy(true);setMessage('');try{onLogin(await usePasskey(username));}catch(e){setMessage(requestErrorMessage(e));}finally{setBusy(false);}}}>Sign in with a passkey</button>}</form></main>;
}

function Detail({item,refresh,curation,accessManagement,onEdit,onClose,onSessionExpired}:{item:Item;refresh:number;curation:boolean;accessManagement:boolean;onEdit:()=>void;onClose:()=>void;onSessionExpired:()=>void}){
 const [evidence,setEvidence]=useState<{id:string;kind:string;outcome:string}[]>([]);const [error,setError]=useState('');const [loading,setLoading]=useState(true);const [retry,setRetry]=useState(0);
 useEffect(()=>{const controller=new AbortController();setEvidence([]);setError('');setLoading(true);api<typeof evidence>(`/items/${item.id}/evidence`,{signal:controller.signal}).then(value=>{if(!controller.signal.aborted)setEvidence(value);}).catch(e=>{if(controller.signal.aborted)return;if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}).finally(()=>{if(!controller.signal.aborted)setLoading(false);});return()=>controller.abort();},[item.id,item.revision,refresh,retry]);
 const [queueNotice,setQueueNotice]=useState('');
 const presentation=item.presentation;
 return <aside className="detail" aria-label={`${item.title} details`}><button className="close" onClick={onClose} aria-label="Close details">×</button><Artwork item={item} large/><h2>{item.title}</h2><p>{item.subtitle}</p><dl><dt>Physical copies</dt><dd>{item.specimens.length}</dd><dt>Captures</dt><dd>{item.captures.length}</dd><dt>Identity</dt><dd>{item.identity}</dd></dl>{curation?<button onClick={onEdit}>Edit / attach files</button>:accessManagement&&<button onClick={onEdit}>Manage collection access</button>}<MusicTracks key={item.id} item={item} canPrepare={false} onSessionExpired={onSessionExpired}/>{presentation&&<section aria-label="Module details"><h3>Module details</h3>{presentation.fields.map(field=><p key={field.label}><strong>{field.label}:</strong> {field.value}</p>)}{presentation.badges.map(badge=><span className={`identity ${badge.tone}`} key={badge.label}>{badge.label}</span>)}{presentation.limitations.map(value=><p className="muted" key={value}>{value}</p>)}{!presentation.available&&presentation.values.map(value=><details key={value.label}><summary>{value.label}</summary><pre>{JSON.stringify(value.value,null,2)}</pre></details>)}</section>}<h3>Evidence</h3>{loading?<p role="status">Loading evidence…</p>:error?<div role="alert" className="error">Evidence could not be loaded. {error}<button onClick={()=>setRetry(v=>v+1)}>Retry evidence</button></div>:!evidence.length?<p className="muted">No evidence recorded.</p>:evidence.map(e=><p className="evidence" key={e.id}><span>{e.kind.replaceAll('_',' ')}</span><strong>{e.outcome}</strong></p>)}<h3>Stored files</h3>{queueNotice&&<p role="status">{queueNotice}</p>}{item.captures.flatMap(c=>c.representations).flatMap(r=>r.files.map((f,i)=>{const url=`/api/v1/items/${item.id}/representations/${r.id}/${i}`;return <div className="stored-file" key={`${r.id}/${i}`}><a href={url} download={f.name}>{f.name}</a><small>{(f.bytes/1048576).toFixed(1)} MiB · {r.role==='preservation_master'?'Original':r.role==='lossless_derivative'?'Lossless copy':'Lossy copy'}</small>{r.role==='preservation_master'&&audioMediaType(f.name)&&<button onClick={()=>{const count=addMusicTracks([{itemId:item.id,collectionId:item.collection_id,album:item.title,title:f.name,position:i+1,representationId:r.id,fileIndex:i,mediaType:audioMediaType(f.name)!,artwork:item.primary_artwork||undefined,original:true}]);setQueueNotice(count?'Added file to your queue.':'Your queue is full. Remove tracks to add more.');}}>Add file to queue</button>}</div>;}))}</aside>;
}

createRoot(document.getElementById('root')!).render(<App/>);
