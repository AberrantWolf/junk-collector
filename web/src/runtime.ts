import {AuthenticationCoordinator,type AuthenticationHandler,type AuthenticationIdentity} from './authentication-coordinator';
import React,{useEffect,useRef,useState,useId} from 'react';

export {React as default,useState,useEffect,useRef,useId};

export class ApiRequestError extends Error{
 readonly status:number;
 readonly code:string;
 constructor(status:number,code:string,message:string){super(message);this.name='ApiRequestError';this.status=status;this.code=code;}
}

export function isApiRequestError(error:unknown):error is ApiRequestError{
 return error instanceof ApiRequestError;
}

export function requestErrorMessage(error:unknown,forbiddenMessage?:string):string{
 if(isApiRequestError(error)&&error.code==='forbidden'&&forbiddenMessage)return forbiddenMessage;
 if(error instanceof Error)return error.message;
 return 'The request could not be completed.';
}

let authentication:AuthenticationCoordinator|undefined;
let identity:AuthenticationIdentity|undefined;
export function getApiIdentity(){return identity?{...identity}:undefined;}
export function setApiIdentity(value?:AuthenticationIdentity){
 if(identity?.user_id!==value?.user_id){authentication?.cancel();window.dispatchEvent(new CustomEvent('collector-api-identity-changed',{detail:{userId:value?.user_id}}));}
 identity=value?{...value}:undefined;
}
export function setRecentAuthenticationHandler(handler?:AuthenticationHandler){authentication?.cancel();authentication=handler?new AuthenticationCoordinator(handler):undefined;}
export type ApiRequestInit=RequestInit&{authenticationPolicy?:'retry'|'review'};

export async function api<T=unknown>(path:string,init:ApiRequestInit={}):Promise<T>{
 // Snapshot mutable request containers before any asynchronous continuation.
 let body=init.body;
 if(body instanceof URLSearchParams)body=new URLSearchParams(body);
 else if(body instanceof ArrayBuffer)body=body.slice(0);
 else if(ArrayBuffer.isView(body))body=new Uint8Array(body.buffer,body.byteOffset,body.byteLength).slice();
 else if(body instanceof FormData){const copy=new FormData();for(const [key,value] of body.entries())copy.append(key,value);body=copy;}
 const request={...init,body,headers:new Headers(init.headers)};
 return performRequest<T>(path,request,false,identity?{...identity}:undefined);
}

async function performRequest<T>(path:string,init:ApiRequestInit,retried:boolean,submittedIdentity?:AuthenticationIdentity):Promise<T>{
 init.signal?.throwIfAborted();
 const method=(init.method||'GET').toUpperCase();
 const needsCommandKey=!['GET','HEAD','OPTIONS'].includes(method)&&!path.startsWith('/auth/')&&!/^\/uploads\/[^/]+\/files\//.test(path);
 const headers=new Headers(init.headers);
 if(submittedIdentity&&!path.startsWith('/auth/'))headers.set('X-Collector-Actor',submittedIdentity.user_id);
 if(init.body&&typeof init.body==='string'&&!headers.has('Content-Type'))headers.set('Content-Type','application/json');
 if(needsCommandKey&&!headers.has('Idempotency-Key'))headers.set('Idempotency-Key',crypto.randomUUID());
 const {authenticationPolicy='retry',...transport}=init;
 const request:RequestInit={credentials:'same-origin',...transport,headers};
 let response:Response;
 try{response=await fetch(`/api/v1${path}`,request);}catch(error){if(!needsCommandKey||init.signal?.aborted)throw error;response=await fetch(`/api/v1${path}`,request);}
 if(!response.ok){
  let code=`http_${response.status}`;let message=`Request failed (${response.status})`;
  try{const body=await response.json() as {error?:{code?:string;message?:string}};code=body.error?.code||code;message=body.error?.message||message;}catch{}
  if(code==='recent_authentication_required'&&!retried&&authentication&&!path.startsWith('/auth/')){
   if(!submittedIdentity)throw new Error('Sign in again before resubmitting this action.');
   await authentication.wait(submittedIdentity,init.signal??undefined);
   init.signal?.throwIfAborted();
   const verified=await performRequest<AuthenticationIdentity>('/me',{signal:init.signal},true,submittedIdentity);
   if(verified.user_id!==submittedIdentity.user_id||identity?.user_id!==submittedIdentity.user_id)throw new Error('Account changed. The pending action was not submitted.');
   if(authenticationPolicy==='review')throw new ApiRequestError(409,'authentication_review_required','Authentication changed the session. Review the refreshed sessions and choose the action again.');
   if(init.body instanceof ReadableStream)throw new Error('This upload must be resubmitted after authentication.');
   return performRequest<T>(path,{...request,authenticationPolicy},true,submittedIdentity);
  }
  throw new ApiRequestError(response.status,code,message);
 }
 const result=response.status===204?undefined as T:await response.json() as T;init.signal?.throwIfAborted();
 if(submittedIdentity&&identity?.user_id!==submittedIdentity.user_id)throw new Error('Account changed. Reopen this task before continuing.');
 return result;
}

export {useMusicQueue,addMusicTracks,startMusicTracks} from './music-queue';
export type {QueuedTrack} from './music-queue';

export {OperationScope} from './operation-scope';
export {useOperationScope} from './use-operation-scope';

export {registerPasskey} from './passkeys';
