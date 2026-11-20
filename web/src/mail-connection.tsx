import React,{useEffect,useRef,useState,api,requestErrorMessage,isApiRequestError,retainUnchanged} from '@collector/runtime';
import type {components} from './generated/api';
type Profile=components['schemas']['MailProfile'];
type Connection=components['schemas']['MailConnection'];
type Setup=components['schemas']['MailSetupStatus'];
type Props={userId:string;collection:string;canCurate:boolean;canAdminister:boolean;onSessionExpired:()=>void};
export default function MailConnection({userId,collection,canCurate,canAdminister,onSessionExpired}:Props){
 const [profiles,setProfiles]=useState<Profile[]>([]),[connections,setConnections]=useState<Connection[]>([]),[setup,setSetup]=useState<Setup>(),[error,setError]=useState(''),[loadError,setLoadError]=useState(''),[busy,setBusy]=useState(false),[refresh,setRefresh]=useState(0),[loaded,setLoaded]=useState(false),[username,setUsername]=useState(''),[password,setPassword]=useState(''),[folders,setFolders]=useState('INBOX');
 const mutation=useRef<AbortController|null>(null),receipt=useRef(crypto.randomUUID());
 const [credentialVersion,setCredentialVersion]=useState(0);
 const setupFolderInputId=React.useId();
 function fail(e:unknown){if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}
 useEffect(()=>{const c=new AbortController();let timer:ReturnType<typeof setTimeout>|undefined;
  async function poll(){try{
   const results=await Promise.allSettled([api<Profile[]>('/mail/profiles',{signal:c.signal,operation:'Load mail profiles'}),api<Connection[]>('/mail/connections',{signal:c.signal,operation:'Load mail connections'}),canAdminister?api<Setup>('/mail/setup',{signal:c.signal,operation:'Load Bridge status'}):Promise.resolve(undefined)]);
   if(c.signal.aborted)return;
   const [p,cx,s]=results;
   if(p.status==='fulfilled')setProfiles(current=>retainUnchanged(current,p.value));
   if(cx.status==='fulfilled')setConnections(current=>retainUnchanged(current,cx.value));
   if(s.status==='fulfilled')setSetup(current=>retainUnchanged(current,s.value));else setSetup(undefined);
   if(p.status==='fulfilled'&&cx.status==='fulfilled')setLoaded(true);
   const failures=results.filter((v):v is PromiseRejectedResult=>v.status==='rejected');
   const expired=failures.find(v=>isApiRequestError(v.reason)&&v.reason.code==='unauthenticated');
   if(expired)fail(expired.reason);else setLoadError(failures.map(v=>requestErrorMessage(v.reason)).join('; '));
  }catch(e){if(!c.signal.aborted)setLoadError(requestErrorMessage(e));}finally{if(!c.signal.aborted)timer=setTimeout(()=>void poll(),5000);}}
  if(!busy)void poll();return()=>{c.abort();clearTimeout(timer);};
 },[userId,collection,canAdminister,busy,refresh]);
 async function connect(e:React.FormEvent){e.preventDefault();setError('');setBusy(true);
  // Mutations have their own lifecycle; polling is paused while secrets are submitted.
  const c=new AbortController();mutation.current=c;
  try{await api('/mail/setup',{method:'POST',signal:c.signal,headers:{'Idempotency-Key':receipt.current},body:JSON.stringify({username,password,folders:folders.split('\n').filter(v=>v.trim().length>0)})});if(!c.signal.aborted){setPassword('');setUsername('');setRefresh(v=>v+1);}}
  catch(e){if(!c.signal.aborted)fail(e);}finally{if(!c.signal.aborted)setBusy(false);}
 }
 async function toggle(p:Profile){const current=connections.find(c=>c.profile.id===p.id);setError('');setBusy(true);const c=new AbortController();mutation.current=c;
  try{await api(`/mail/connections/${p.id}`,{method:'PUT',signal:c.signal,body:JSON.stringify({collection_id:collection,enabled:!current?.enabled,expected_revision:current?.revision||0})});if(!c.signal.aborted)setRefresh(v=>v+1);}catch(e){if(!c.signal.aborted)fail(e);}finally{if(!c.signal.aborted)setBusy(false);}
 }
 async function saveFolders(p:Profile,current:Connection|undefined,folders:string[],key:string){setError('');setBusy(true);const c=new AbortController();mutation.current=c;
  try{await api(`/mail/connections/${p.id}`,{method:'PUT',signal:c.signal,headers:{'Idempotency-Key':key},body:JSON.stringify({collection_id:collection,enabled:current?.enabled??false,expected_revision:current?.revision||0,folders})});if(c.signal.aborted)return false;setRefresh(v=>v+1);return true;}catch(e){if(!c.signal.aborted)fail(e);return false;}finally{if(!c.signal.aborted)setBusy(false);}
 }
 async function rescan(p:Profile,current:Connection){setError('');setBusy(true);const c=new AbortController();mutation.current=c;
  try{await api(`/mail/connections/${p.id}`,{method:'PUT',signal:c.signal,body:JSON.stringify({collection_id:collection,enabled:current.enabled,expected_revision:current.revision,rescan:true})});if(!c.signal.aborted)setRefresh(v=>v+1);}catch(e){if(!c.signal.aborted)fail(e);}finally{if(!c.signal.aborted)setBusy(false);}
 }
 async function repair(p:Profile,command:components['schemas']['RepairBridge'],key:string){setError('');setBusy(true);const c=new AbortController();mutation.current=c;
  try{await api(`/mail/profiles/${p.id}/repair`,{method:'POST',signal:c.signal,headers:{'Idempotency-Key':key},body:JSON.stringify(command)});if(c.signal.aborted)return false;setRefresh(v=>v+1);return true;}catch(e){if(!c.signal.aborted)fail(e);return false;}finally{if(!c.signal.aborted)setBusy(false);}
 }
 useEffect(()=>()=>mutation.current?.abort(),[userId,collection]);
 function edit(update:()=>void){update();receipt.current=crypto.randomUUID();}
 function editCredentials(update:()=>void){edit(update);setCredentialVersion(v=>v+1);}
 function findSetupFolders(signal:AbortSignal){
  const credentials:components['schemas']['DiscoverBridgeFolders']={username,password};
  return api<string[]>('/mail/setup/folders',{method:'POST',signal,body:JSON.stringify(credentials)});
 }
 const bridge=setup?.bridge;
 const staleProgress=bridge?.event_at!=null&&Date.now()/1000-bridge.event_at>120;
 const phase=bridge?({starting:'Starting',unknown:'Running',syncing:staleProgress?'Syncing (no recent update)':'Syncing mailbox',synced:'Synced',interrupted:'Sync interrupted, will retry',stopped:'Stopped'}[bridge.phase]):'Status unavailable';
 return <section className="mail-connection" aria-label="Mail connection"><header><h2>Connect your email</h2><p>Connect Proton Bridge, then turn on receipt scanning for a collection. Bridge keeps an encrypted copy of your whole mailbox on this server.</p></header>
 {canAdminister&&<section className="bridge-status" aria-label="Proton Bridge status"><h3>Proton Bridge</h3><p role="status">{phase}</p>{bridge?.percent!=null&&bridge.phase==='syncing'&&<><progress aria-label="Bridge mailbox sync" max={100} value={bridge.percent}/><p>{bridge.percent.toFixed(1)}%{bridge.remaining_seconds!=null&&!staleProgress?` · Estimated ${Math.ceil(bridge.remaining_seconds/60)} min remaining`:''}</p></>}{bridge?.event_at&&<p className="muted">Last update: {new Date(bridge.event_at*1000).toLocaleString()}</p>}{setup?.problem&&<p role="alert">{setup.problem}</p>}</section>}
 {!loaded&&!error&&<p role="status">Loading…</p>}
 {loaded&&!profiles.length&&(canAdminister?<form className="bridge-connect-form" onSubmit={e=>void connect(e)}><h3>Connect Proton Bridge</h3><p>In Bridge’s CLI on the server, sign in with <code>login</code>, then run <code>info</code> to see the local IMAP username and password.</p><p>Enter the local IMAP password here, not your Proton password.</p><fieldset disabled={busy}><legend>Local Bridge connection</legend><label>Bridge IMAP username<input autoComplete="off" maxLength={320} required value={username} onChange={e=>editCredentials(()=>setUsername(e.target.value))}/></label><label>Bridge IMAP password<input type="password" autoComplete="off" maxLength={1024} required value={password} onChange={e=>editCredentials(()=>setPassword(e.target.value))}/></label><label htmlFor={setupFolderInputId}>Mail folders, one per line</label><textarea id={setupFolderInputId} rows={3} maxLength={4111} required value={folders} onChange={e=>edit(()=>setFolders(e.target.value))}/><MailboxPicker key={`${userId}:${credentialVersion}:${bridge?.imap_port}:${bridge?.certificate_sha256}`} folders={folders} disabled={!setup?.can_connect||!username||!password} load={findSetupFolders} onChange={value=>edit(()=>setFolders(value))} onSessionExpired={onSessionExpired}/>{bridge?.certificate_sha256&&<p className="mail-certificate">Bridge certificate fingerprint: <code>{bridge.certificate_sha256}</code></p>}<button type="submit" disabled={!setup?.can_connect}>Connect Bridge</button></fieldset>{!setup?.can_connect&&<p>Available once Bridge is running.</p>}</form>:<p>A server administrator needs to connect your mail first.</p>)}
 {profiles.map(p=>{const c=connections.find(v=>v.profile.id===p.id);return <section key={p.id} aria-label="Receipt scan"><h3>{p.label} · Connected</h3><p>Receipt scanning: {c?.enabled?'Enabled':'Paused'}{c?.last_checked?` · Last checked ${new Date(c.last_checked).toLocaleString()}`:''}</p>{c?.problem&&(c.profile.configuration_revision!==p.configuration_revision?<p role="status">Settings changed since the last problem. Waiting for the next scan.</p>:<p role="alert">Last scan problem: {c.problem}{c.failure?.folder?` Folder: ${c.failure.folder}`:''}</p>)}<ScanHealth profile={p} connection={c}/>{canAdminister&&<RepairConnection key={`${userId}:${p.id}`} profile={p} certificate={bridge?.certificate_sha256??undefined} disabled={busy||!setup?.can_connect} onSave={repair}/>}{c&&c.collection_id!==collection?<p>This connection is used by another collection. Select that collection to manage it.</p>:<>{(!collection||!canCurate)&&<p>{!collection?'Select a collection to turn on receipt scanning.':'You need Curator access to this collection.'}</p>}<FolderEditor key={`${userId}:${collection}:${p.id}`} profile={p} connection={c} disabled={busy||!collection||!canCurate} onSave={saveFolders} onSessionExpired={onSessionExpired}/>{c&&<button disabled={busy||!canCurate} onClick={()=>void rescan(p,c)}>Rescan configured folders</button>}<button disabled={busy||!collection||!canCurate} onClick={()=>void toggle(p)}>{c?.enabled?'Pause receipt scanning':'Enable receipt scanning'}</button></>}</section>;})}
 <button disabled={busy} onClick={()=>{setError('');setRefresh(v=>v+1);}}>Refresh mail status</button>{busy&&<p role="status">Saving mail connection…</p>}{loadError&&<p role="alert" className="notice error">{loadError}</p>}{error&&<p role="alert" className="notice error">{error}</p>}
 </section>;
}

function FolderEditor({profile,connection,disabled,onSave,onSessionExpired}:{profile:Profile;connection:Connection|undefined;disabled:boolean;onSave:(p:Profile,c:Connection|undefined,folders:string[],key:string)=>Promise<boolean>;onSessionExpired:()=>void}){
 const folderInputId=React.useId();
 const [draft,setDraft]=useState<{text:string;connection:Connection|undefined;key:string}>();
 const saved=connection?.folders??profile.folders??[];
 if(!draft)return <><p>Scanned folders: <code>{saved.join(', ')}</code></p><button disabled={disabled} onClick={()=>setDraft({text:saved.join('\n'),connection,key:crypto.randomUUID()})}>Edit scanned folders</button></>;
 function updateFolders(text:string){if(draft)setDraft({...draft,text,key:crypto.randomUUID()});}
 async function save(e:React.FormEvent){e.preventDefault();if(!draft)return;const ok=await onSave(profile,draft.connection,draft.text.split('\n').filter(v=>v.trim().length>0),draft.key);if(ok)setDraft(undefined);}
 return <form className="bridge-connect-form" onSubmit={e=>void save(e)}><fieldset disabled={disabled}><legend>Scanned mail folders</legend>{!draft.connection&&<p>Saving links this connection to the selected collection. Scanning stays paused until you enable it.</p>}<label htmlFor={folderInputId}>Mail folders, one per line</label><textarea id={folderInputId} rows={4} maxLength={4111} required value={draft.text} onChange={e=>updateFolders(e.target.value)}/><MailboxPicker key={profile.configuration_revision} folders={draft.text} load={signal=>api<string[]>(`/mail/profiles/${profile.id}/folders`,{signal})} onChange={updateFolders} onSessionExpired={onSessionExpired}/><p>Up to 16 folders. New folders are scanned from the start.</p><button type="submit">Save folders</button><button type="button" onClick={()=>setDraft(undefined)}>Cancel folder editing</button></fieldset></form>;
}

function MailboxPicker({folders,disabled,load,onChange,onSessionExpired}:{
 folders:string;disabled?:boolean;load:(signal:AbortSignal)=>Promise<string[]>;onChange:(folders:string)=>void;onSessionExpired:()=>void;
}){
 const request=useRef<AbortController|null>(null);
 const choiceId=React.useId();
 const [choices,setChoices]=useState<string[]>(),[selected,setSelected]=useState(''),[loading,setLoading]=useState(false),[error,setError]=useState('');
 const configured=folders.split('\n').filter(name=>name.trim().length>0);
 const available=choices?.filter(name=>!configured.includes(name));
 const choice=available?.includes(selected)?selected:'';
 const canAdd=configured.length<16;
 useEffect(()=>()=>request.current?.abort(),[]);
 async function discover(){
  request.current?.abort();
  const controller=new AbortController();
  request.current=controller;
  setChoices(undefined);setSelected('');setError('');setLoading(true);
  try{
   const names=await load(controller.signal);
   if(!controller.signal.aborted)setChoices(names);
  }catch(e){
   if(controller.signal.aborted)return;
   if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();
   else setError(requestErrorMessage(e));
  }finally{
   if(!controller.signal.aborted)setLoading(false);
  }
 }
 return <>
  <button type="button" disabled={disabled||loading} onClick={()=>void discover()}>{choices?'Refresh mail folders':'Find mail folders'}</button>
  {loading&&<p role="status">Finding folders in Bridge…</p>}
  {error&&<p role="alert">{error} You can still type folder names.</p>}
  {available&&available.length>0&&<>
   <label htmlFor={choiceId}>Available mail folders</label>
   <select id={choiceId} value={choice} onChange={e=>setSelected(e.target.value)}>
    <option value="">Choose a folder</option>
    {available.map(name=><option key={name} value={name}>{name}</option>)}
   </select>
   <button type="button" disabled={!choice||!canAdd} onClick={()=>{onChange([...configured,choice].join('\n'));setSelected('');}}>Add folder</button>
   {!canAdd&&<p role="status">You can scan up to 16 folders.</p>}
  </>}
  {available?.length===0&&<p role="status">{choices?.length?'All listed folders are already included.':'Bridge returned no selectable folders.'}</p>}
 </>;
}

function ScanHealth({profile,connection}:{profile:Profile;connection:Connection|undefined}){
 const folders=connection?.folders??profile.folders??[];
 return <section aria-label="Receipt history progress"><h4>Receipt history</h4>{folders.map(folder=>{const p=connection?.progress?.find(p=>p.folder===folder);return <p key={folder}><code>{folder}</code>: {!p?'Not checked yet':p.caught_up?'Up to date':`Scanning · up to ${p.available_through-p.scanned_through} messages left`}{p?` · Checked ${new Date(p.checked_at).toLocaleString()}`:''}</p>;})}<p className="muted">Rescan to go through folder history again. Existing suggestions are kept.</p></section>;
}
function RepairConnection({profile,certificate,disabled,onSave}:{profile:Profile;certificate:string|undefined;disabled:boolean;onSave:(p:Profile,c:components['schemas']['RepairBridge'],key:string)=>Promise<boolean>}){
 const [draft,setDraft]=useState<{username:string;password:string;certificate:string;revision:number;confirmed:boolean;key:string}>();
 const usernameId=React.useId(),passwordId=React.useId();
 if(!draft)return <button disabled={disabled||!certificate} onClick={()=>setDraft({username:'',password:'',certificate:certificate!,revision:profile.configuration_revision||0,confirmed:false,key:crypto.randomUUID()})}>Revalidate Bridge connection</button>;
 async function save(e:React.FormEvent){e.preventDefault();if(!draft?.confirmed)return;const ok=await onSave(profile,{username:draft.username,password:draft.password,certificate_sha256:draft.certificate,expected_revision:draft.revision},draft.key);if(ok)setDraft(undefined);}
 return <form className="bridge-connect-form" onSubmit={e=>void save(e)}><fieldset disabled={disabled}><legend>Revalidate Bridge connection</legend><p>Enter the current local IMAP username and password from the Bridge terminal. Scan history and settings are kept.</p><label htmlFor={usernameId}>Current Bridge IMAP username</label><input id={usernameId} required maxLength={320} autoComplete="off" value={draft.username} onChange={e=>setDraft({...draft,username:e.target.value,key:crypto.randomUUID()})}/><label htmlFor={passwordId}>Current Bridge IMAP password</label><input id={passwordId} type="password" required maxLength={1024} autoComplete="off" value={draft.password} onChange={e=>setDraft({...draft,password:e.target.value,key:crypto.randomUUID()})}/><p>Certificate fingerprint to trust: <code>{draft.certificate}</code></p><label className="bridge-confirmation"><input type="checkbox" checked={draft.confirmed} onChange={e=>setDraft({...draft,confirmed:e.target.checked})}/>I checked this fingerprint in the Bridge terminal.</label><button type="submit" disabled={!draft.confirmed}>Save revalidated connection</button><button type="button" onClick={()=>setDraft(undefined)}>Cancel revalidation</button></fieldset></form>;
}
