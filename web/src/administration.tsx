import React,{useEffect,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';

type Feedback={kind:'success'|'error';text:string};
type Settings={exposure:{public_administration:boolean;public_curation:boolean};server_id:string};

export default function Administration({onClose,onSessionExpired}:{onClose:()=>void;onSessionExpired:()=>void}){
 const [name,setName]=useState('');const [feedback,setFeedback]=useState<Feedback>();const [settings,setSettings]=useState<Settings>();const [busy,setBusy]=useState(false);
 function fail(error:unknown){
  if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}
  setFeedback({kind:'error',text:requestErrorMessage(error,'You are signed in, but this operation requires a server administrator. Collection ownership alone does not grant server administration.')});
 }
 useEffect(()=>{api<Settings>('/admin/settings').then(setSettings).catch(fail);},[]);
 async function createCollection(){
  setBusy(true);setFeedback(undefined);try{await api('/admin/collections',{method:'POST',body:JSON.stringify({name})});setName('');setFeedback({kind:'success',text:'Collection created. Close this panel to browse it.'});}catch(error){fail(error);}finally{setBusy(false);}
 }
 return <div className="modal-backdrop"><section className="modal" role="dialog" aria-modal="true" aria-labelledby="admin-title" aria-busy={busy}><button className="close" onClick={onClose} aria-label="Close administration">×</button><p className="eyebrow">Server administration</p><h2 id="admin-title">Make room for a collection.</h2><form onSubmit={event=>{event.preventDefault();void createCollection();}}><label>Collection name<input required disabled={busy||!settings} value={name} onChange={event=>setName(event.target.value)} placeholder="Games on the shelf"/></label><button className="primary" disabled={busy||!settings}>{busy?'Creating…':'Create collection'}</button></form><hr/><h3>Public access</h3>{settings?<dl><dt>Client browsing / media</dt><dd>Enabled</dd><dt>Collection curation</dt><dd>{settings.exposure.public_curation||settings.exposure.public_administration?'Enabled':'Private only'}</dd><dt>Administration</dt><dd>{settings.exposure.public_administration?'Enabled':'Private only'}</dd></dl>:<p className="muted">Loading authorized server settings…</p>}<p className="muted">Exposure is configured in the server configuration and applied at startup. Account and collection permissions still apply.</p>{feedback&&<p role={feedback.kind==='error'?'alert':'status'} className={`notice ${feedback.kind==='error'?'error':''}`}>{feedback.text}</p>}</section></div>;
}
