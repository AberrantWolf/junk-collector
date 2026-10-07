import {api} from './runtime';
export async function registerPasskey(){
 const started=await api<{ceremony_id:string;options:{publicKey:PublicKeyCredentialCreationOptionsJSON}}>('/auth/passkeys/register/start',{method:'POST'});
 const publicKey=PublicKeyCredential.parseCreationOptionsFromJSON(started.options.publicKey);
 const credential=await navigator.credentials.create({publicKey}) as PublicKeyCredential|null;
 if(!credential)throw new Error('Passkey registration was cancelled.');
 await api('/auth/passkeys/register/finish',{method:'POST',body:JSON.stringify({ceremony_id:started.ceremony_id,credential:credential.toJSON()})});
}
