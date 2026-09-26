import React,{useEffect,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import {useDialogDismiss} from './dialog';
import type {components} from './generated/api';

type Account=components['schemas']['AccountView'];
type Session=components['schemas']['SessionView'];
type Passkey=components['schemas']['PasskeyView'];
type NativeDevice=components['schemas']['NativeDeviceView'];

export default function Security({onClose,onSessionExpired}:{onClose:()=>void;onSessionExpired:()=>void}){
 const dialog=useDialogDismiss(()=>{if(codes&&!window.confirm('Have you saved these one-time recovery codes? They cannot be shown again after closing.'))return;onClose();});
 const [profile,setProfile]=useState<Account>();const [displayName,setDisplayName]=useState('');const [sessions,setSessions]=useState<Session[]>([]);const [passkeys,setPasskeys]=useState<Passkey[]>([]);const [devices,setDevices]=useState<NativeDevice[]>([]);const [codes,setCodes]=useState<string[]>();const [message,setMessage]=useState('');const [busy,setBusy]=useState(false);
 function fail(error:unknown){if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}setMessage(requestErrorMessage(error));}
 async function load(){try{const [account,currentSessions,currentPasskeys,currentDevices]=await Promise.all([api<Account>('/me/profile'),api<Session[]>('/security/sessions'),api<Passkey[]>('/security/passkeys'),api<NativeDevice[]>('/security/native-devices')]);setProfile(account);setDisplayName(account.display_name);setSessions(currentSessions);setPasskeys(currentPasskeys);setDevices(currentDevices);}catch(error){fail(error);}}
 useEffect(()=>{void load();},[]);
 async function updateProfile(){if(!profile)return;setBusy(true);setMessage('');try{const account=await api<Account>('/me/profile',{method:'PUT',body:JSON.stringify({expected_revision:profile.revision,display_name:displayName})});setProfile(account);setMessage('Profile updated.');}catch(error){fail(error);}finally{setBusy(false);}}
 async function replaceCodes(){setBusy(true);setCodes(undefined);setMessage('');try{setCodes(await api<string[]>('/security/recovery-codes',{method:'POST'}));setMessage('Save these codes now. They will not be shown again.');}catch(error){fail(error);}finally{setBusy(false);}}
 async function copyCodes(){if(!codes)return;try{await navigator.clipboard.writeText(codes.join('\n'));setMessage('Recovery codes copied. Store them somewhere private.');}catch{setMessage('Copy failed. Select and save the displayed codes manually.');}}
 function downloadCodes(){if(!codes)return;const url=URL.createObjectURL(new Blob([codes.join('\n')+'\n'],{type:'text/plain'}));const link=document.createElement('a');link.href=url;link.download='junk-collector-recovery-codes.txt';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);setMessage('Recovery codes downloaded. Store the file somewhere private.');}
 async function revokeSession(session:Session){setBusy(true);try{await api(`/devices/${session.id}`,{method:'DELETE'});if(session.current){try{await api('/auth/logout',{method:'POST'});}catch(error){if(!isApiRequestError(error)||error.code!=='unauthenticated')throw error;}onSessionExpired();return;}await load();}catch(error){fail(error);}finally{setBusy(false);}}
 async function revokePasskey(passkey:Passkey){setBusy(true);try{await api(`/security/passkeys/${passkey.id}?expected_revision=${passkey.revision}`,{method:'DELETE'});await load();}catch(error){fail(error);}finally{setBusy(false);}}
 async function revokeDevice(device:NativeDevice){setBusy(true);try{await api(`/security/native-devices/${device.id}?expected_revision=${device.revision}`,{method:'DELETE'});await load();}catch(error){fail(error);}finally{setBusy(false);}}
 return <div className="modal-backdrop"><section ref={dialog.ref} data-unsaved={!!profile&&displayName!==profile.display_name} className="modal" role="dialog" aria-modal="true" aria-labelledby="security-title" aria-busy={busy}><button className="close" onClick={dialog.dismiss} aria-label="Close account and security">×</button><p className="eyebrow">Account &amp; security</p><h2 id="security-title">Your durable profile.</h2>
  {profile&&<form onSubmit={event=>{event.preventDefault();void updateProfile();}}><label>Display name<input value={displayName} onChange={event=>setDisplayName(event.target.value)} disabled={busy}/></label><button disabled={busy||!displayName.trim()}>Save profile</button></form>}
  <hr/><h3>Recovery codes</h3><p className="muted">Replacing codes invalidates every previous recovery code. A used code replaces your password and browser sessions.</p><button disabled={busy} onClick={replaceCodes}>Generate replacement codes</button>{codes&&<><p className="notice">Save all codes before closing. Each code works once and this list cannot be retrieved later.</p><button type="button" onClick={()=>void copyCodes()}>Copy codes</button><button type="button" onClick={downloadCodes}>Download codes</button><ol aria-label="Recovery codes">{codes.map(code=><li key={code}><code>{code}</code></li>)}</ol></>}
  <hr/><h3>Browser sessions</h3>{sessions.map(session=><p key={session.id} className="stored-file"><span>{session.label}{session.current?' · Current':''}</span><button disabled={busy} onClick={()=>revokeSession(session)}>Revoke</button></p>)}
  <h3>Passkeys</h3>{passkeys.length?passkeys.map(passkey=><p key={passkey.id} className="stored-file"><span>{passkey.label}</span><button disabled={busy} onClick={()=>revokePasskey(passkey)}>Revoke</button></p>):<p className="muted">No passkeys registered.</p>}
  <h3>Paired devices</h3>{devices.length?devices.map(device=><p key={device.id} className="stored-file"><span>{device.label}</span><button disabled={busy} onClick={()=>revokeDevice(device)}>Revoke</button></p>):<p className="muted">No native clients are paired.</p>}
  {message&&<p role="status" className="notice">{message}</p>}
 </section></div>;
}
