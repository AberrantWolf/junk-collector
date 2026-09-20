import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from './runtime';
import {createRoot} from 'react-dom/client';
import type {components} from './generated/api';
import MusicTracks from './music-tracks';

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
 const artwork=item.assets.find(asset=>asset.id===item.primary_artwork);const [failed,setFailed]=useState(false);
 useEffect(()=>setFailed(false),[artwork?.id]);
 return <div className="artwork-stack"><div className={`artwork ${large?'large':''}`}>{artwork&&!failed?<img src={`/api/v1/collections/${item.collection_id}/assets/${artwork.id}/thumbnail`} loading="lazy" alt={artwork.alt_text||`${item.title} artwork`} onError={()=>setFailed(true)}/>:<span role="img" aria-label="No artwork attached" className="art-placeholder"><b aria-hidden="true">◇</b><small>No art</small></span>}</div>{item.effective_metadata.resolution==='tentative'&&<small className="metadata-state">Tentative metadata</small>}</div>;
}

function App(){
 const [features,setFeatures]=useState<Features>();const [actor,setActor]=useState<Principal>();const [ready,setReady]=useState(false);
 const [collections,setCollections]=useState<Collection[]>([]);const [collectionTypes,setCollectionTypes]=useState<CollectionType[]>([]);const [collection,setCollection]=useState('');const [search,setSearch]=useState('');const [items,setItems]=useState<Item[]>([]);const [cursor,setCursor]=useState<string|null>(null);const [selected,setSelected]=useState<Item>();const [error,setError]=useState('');const [loginMessage,setLoginMessage]=useState('');const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false);const [refresh,setRefresh]=useState(0);
 useEffect(()=>{function escape(event:KeyboardEvent){if(event.key==='Escape'&&!document.querySelector('[role="dialog"]')){managementRequest.current?.abort();setSelected(undefined);}}document.addEventListener('keydown',escape);return()=>document.removeEventListener('keydown',escape);},[]);
 const [management,setManagement]=useState<React.ReactNode>(null);const generation=useRef(0);const managementRequest=useRef<AbortController|null>(null);

 function clearSession(message:string){
  managementRequest.current?.abort();generation.current++;setBusy(false);setItems([]);setCollections([]);setCollectionTypes([]);setCollection('');setSelected(undefined);setManagement(null);setError('');setNotice('');setLoginMessage(message);setActor(undefined);
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
 useEffect(()=>{let active=true;if(actor)api<Collection[]>('/collections').then(value=>{if(active){setCollections(value);if(collection&&!value.some(c=>c.id===collection))setCollection('');}}).catch(e=>{if(active)handleFailure(e);});return()=>{active=false;};},[actor,refresh]);
 useEffect(()=>{let active=true;if(actor)api<CollectionType[]>('/collection-types').then(value=>{if(active)setCollectionTypes(value);}).catch(e=>{if(active)handleFailure(e);});return()=>{active=false;};},[actor]);
 useEffect(()=>{if(!actor)return;const current=++generation.current;setBusy(true);const timer=setTimeout(()=>{
  const query=new URLSearchParams({limit:'50'});if(collection)query.set('collection_id',collection);if(search)query.set('search',search);
  api<Page>(`/items?${query}`).then(page=>{if(current===generation.current){setItems(page.items);setCursor(page.next_cursor??null);}}).catch(e=>{if(current===generation.current)handleFailure(e);}).finally(()=>{if(current===generation.current)setBusy(false);});
 },180);return()=>{clearTimeout(timer);generation.current++;};},[actor,collection,search,refresh]);

 async function loadMore(){if(!cursor)return;const current=generation.current;setBusy(true);const query=new URLSearchParams({limit:'50',cursor});if(collection)query.set('collection_id',collection);if(search)query.set('search',search);try{const page=await api<Page>(`/items?${query}`);if(current===generation.current){setItems(previous=>[...previous,...page.items]);setCursor(page.next_cursor??null);}}catch(e){if(current===generation.current)handleFailure(e);}finally{if(current===generation.current)setBusy(false);}}
 async function openManagement(kind:'curation'|'administration'|'security',adding=false){
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
   setManagement(React.createElement(module.default,{collections:kind==='curation'?collections.filter(value=>value.content_permissions.can_edit||(!adding&&value.effective_permissions.can_manage_access)):collections,collectionTypes,initialCollectionId:collections.find(value=>value.id===collection&&value.content_permissions.can_edit)?.id,selected:currentItem,onClose:()=>{controller.abort();setManagement(null);setRefresh(n=>n+1);},onSaved:(item:Item)=>{if(controller.signal.aborted)return;setSelected(item);setRefresh(n=>n+1);},onSessionExpired:()=>{if(!controller.signal.aborted)clearSession('Your session expired or was revoked. Sign in again; the failed operation was not saved.');}}));
  }catch(e){if(!controller.signal.aborted)handleFailure(e);}
 }
 async function signOut(){
  try{await api('/auth/logout',{method:'POST'});clearSession('');}
  catch(e){if(isApiRequestError(e)&&e.code==='unauthenticated')clearSession('');else handleFailure(e);}
 }

 if(!ready)return <div className="loading">Opening your collection…</div>;
 if(!actor)return <Login features={features} error={loginMessage||error} onLogin={value=>{setActor(value);setError('');setLoginMessage('');}}/>;
 const clientOnly=!!features&&!features.curation&&!features.administration;
 const noCollections=collections.length===0;
 const selectedCollection=collections.find(c=>c.id===collection);
 return <div className="shell"><aside className="sidebar"><a className="brand" href="/">Junk<br/>Collector<span className="brand-mark">✳</span></a><p className="sidebar-note">Keep the collection.<br/>Keep the evidence.</p><nav aria-label="Collections"><button className={!collection?'active':''} onClick={()=>setCollection('')}>All collections</button>{collections.map(c=><button key={c.id} className={collection===c.id?'active':''} onClick={()=>setCollection(c.id)}>{c.name}</button>)}</nav><div className="sidebar-bottom">{features?.passkeys&&<button onClick={async()=>{try{await registerPasskey();setNotice('Passkey registered. You can use it on the next sign-in.');}catch(e){handleFailure(e);}}}>Add a passkey</button>}<button onClick={()=>openManagement('security')}>Account &amp; security</button>{features?.curation&&collections.some(value=>value.content_permissions.can_edit||value.effective_permissions.can_manage_access)&&<button onClick={()=>openManagement('curation')}>Manage collections</button>}{features?.administration&&actor.administrator&&<button onClick={()=>openManagement('administration')}>Administration</button>}<button onClick={signOut}>Sign out</button></div></aside>
 <main><header className="page-heading"><div><p className="eyebrow">Your archive</p><h1>{selectedCollection?.name||'Collection'}</h1></div><span className="collection-count">{items.length}{cursor?'+':''} {items.length===1&&!cursor?'entry':'entries'}</span></header>
 {clientOnly&&<div className="notice" role="note"><strong>Client access only at this address.</strong> You can browse and play permitted media here. To create collections or change inventory, open the private management address printed by the server. Never publish that private listener through a public tunnel.</div>}
 {selectedCollection?.content_permissions.message&&<ContentNotice collection={selectedCollection}/>}<div className="toolbar"><label className="search"><span aria-hidden="true">⌕</span><input type="search" value={search} onChange={e=>setSearch(e.target.value)} placeholder="Find a title, artist, or platform" aria-label="Search collection"/></label>{features?.curation&&collections.some(value=>value.content_permissions.can_edit)&&<button className="primary" disabled={!collectionTypes.length} onClick={()=>openManagement('curation',true)}>Add / import</button>}</div>
 {notice&&<div role="status" className="notice">{notice}<button onClick={()=>setNotice('')} aria-label="Dismiss notice">×</button></div>}{error&&<div role="alert" className="notice error">{error}<button onClick={()=>setError('')} aria-label="Dismiss error">×</button></div>}
 <div className="browser"><section aria-label="Collection entries" aria-busy={busy}><div className="list-heading"><span>Title / edition</span><span>Identity</span></div>{!items.length&&!busy?<EmptyState search={search} noCollections={noCollections} canAdminister={!!features?.administration&&actor.administrator} canCurate={collections.some(value=>value.content_permissions.can_edit)} onCreate={()=>openManagement('administration')}/>:items.map(item=>{const type=collectionTypes.find(value=>value.id===item.domain.module);const summary=item.presentation.fields.filter(value=>value.label!=='Collection type').slice(0,2).map(value=>value.value).join(' · ');return <button className={`collection-row ${selected?.id===item.id?'selected':''}`} key={item.id} onClick={()=>{managementRequest.current?.abort();setSelected(item);}}><Artwork item={item}/><span className="row-text"><strong>{item.title}</strong><span>{[item.subtitle,summary].filter(Boolean).join(' · ')||'Edition details not yet added'}</span><small>{type?.collection_type_label||`Unavailable type · ${item.domain.module}`}{item.wishlist?' · Wishlist':''}</small></span><span className={`identity ${item.identity}`}>{item.identity==='tentative'?'Tentative match':item.identity==='unidentified'?'Unidentified':item.identity==='confirmed'?'Confirmed identity':'Selected identity'}</span></button>;})}{cursor&&<button className="load-more" disabled={busy} onClick={loadMore}>Load more</button>}{busy&&<p role="status" className="muted">Loading entries…</p>}</section>
 {selected&&<Detail item={selected} curation={!!features?.curation&&selected.content_permissions.can_edit} accessManagement={!!features?.curation&&!!collections.find(value=>value.id===selected.collection_id)?.effective_permissions.can_manage_access} onEdit={()=>openManagement('curation')} onClose={()=>{managementRequest.current?.abort();setSelected(undefined);}} onSessionExpired={()=>clearSession('Your session expired or was revoked. Sign in again.')}/>}</div></main>{management}</div>;
}

function ContentNotice({collection}:{collection:Collection}){
 return <div className="notice" role="note"><strong>{collection.content_permissions.status==='legacy_unassigned'?'Legacy collection type not assigned.':'Collection content is read-only.'}</strong> {collection.content_permissions.message}{collection.content_permissions.administrator_commands?.map(command=><code className="admin-command" key={command}>{command}</code>)}</div>;
}

function EmptyState({search,noCollections,canAdminister,canCurate,onCreate}:{search:string;noCollections:boolean;canAdminister:boolean;canCurate:boolean;onCreate:()=>void}){
 if(search)return <div className="empty"><span aria-hidden="true">▤</span><h2>No matching entries</h2><p>Try a different title or artist.</p></div>;
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

function Detail({item,curation,accessManagement,onEdit,onClose,onSessionExpired}:{item:Item;curation:boolean;accessManagement:boolean;onEdit:()=>void;onClose:()=>void;onSessionExpired:()=>void}){
 const [evidence,setEvidence]=useState<{id:string;kind:string;outcome:string}[]>([]);const [error,setError]=useState('');
 useEffect(()=>{setEvidence([]);setError('');api<typeof evidence>(`/items/${item.id}/evidence`).then(setEvidence).catch(e=>{if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));});},[item.id,item.revision]);
 const presentation=item.presentation;
 return <aside className="detail" aria-label={`${item.title} details`}><button className="close" onClick={onClose} aria-label="Close details">×</button><Artwork item={item} large/><h2>{item.title}</h2><p>{item.subtitle}</p><dl><dt>Physical copies</dt><dd>{item.specimens.length}</dd><dt>Captures</dt><dd>{item.captures.length}</dd><dt>Identity</dt><dd>{item.identity}</dd></dl>{curation?<button onClick={onEdit}>Edit / attach files</button>:accessManagement&&<button onClick={onEdit}>Manage collection access</button>}<MusicTracks key={item.id} item={item} canPrepare={false} onSessionExpired={onSessionExpired}/>{presentation&&<section aria-label="Module details"><h3>Module details</h3>{presentation.fields.map(field=><p key={field.label}><strong>{field.label}:</strong> {field.value}</p>)}{presentation.badges.map(badge=><span className={`identity ${badge.tone}`} key={badge.label}>{badge.label}</span>)}{presentation.limitations.map(value=><p className="muted" key={value}>{value}</p>)}{!presentation.available&&presentation.values.map(value=><details key={value.label}><summary>{value.label}</summary><pre>{JSON.stringify(value.value,null,2)}</pre></details>)}</section>}<h3>Evidence</h3>{!evidence.length?<p className="muted">No verification evidence recorded.</p>:evidence.map(e=><p className="evidence" key={e.id}><span>{e.kind.replaceAll('_',' ')}</span><strong>{e.outcome}</strong></p>)}{error&&<p role="alert" className="error">{error}</p>}<h3>Stored files</h3>{item.captures.flatMap(c=>c.representations).flatMap(r=>r.files.map((f,i)=>{const url=`/api/v1/items/${item.id}/representations/${r.id}/${i}`;return <div className="stored-file" key={`${r.id}/${i}`}><a href={url} download={f.name}>{f.name}</a><small>{(f.bytes/1048576).toFixed(1)} MiB · {r.role==='preservation_master'?'Original':r.role==='lossless_derivative'?'Lossless copy':'Lossy copy'}</small>{r.role==='preservation_master'&&/\.(flac|mp3|wav|opus|ogg)$/i.test(f.name)&&<audio controls preload="none" src={url}/>}</div>;}))}</aside>;
}

createRoot(document.getElementById('root')!).render(<App/>);
