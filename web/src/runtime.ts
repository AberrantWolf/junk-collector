import React,{useEffect,useRef,useState} from 'react';

export {React as default,useState,useEffect,useRef};

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

export async function api<T=unknown>(path:string,init:RequestInit={}):Promise<T>{
 const method=(init.method||'GET').toUpperCase();
 const needsCommandKey=!['GET','HEAD','OPTIONS'].includes(method)&&!path.startsWith('/auth/')&&!/^\/uploads\/[^/]+\/files\//.test(path);
 const headers=new Headers(init.headers);
 if(init.body&&typeof init.body==='string'&&!headers.has('Content-Type'))headers.set('Content-Type','application/json');
 if(needsCommandKey&&!headers.has('Idempotency-Key'))headers.set('Idempotency-Key',crypto.randomUUID());
 const request:RequestInit={credentials:'same-origin',...init,headers};
 let response:Response;
 try{response=await fetch(`/api/v1${path}`,request);}catch(error){if(!needsCommandKey||init.signal?.aborted)throw error;response=await fetch(`/api/v1${path}`,request);}
 if(!response.ok){
  let code=`http_${response.status}`;let message=`Request failed (${response.status})`;
  try{const body=await response.json() as {error?:{code?:string;message?:string}};code=body.error?.code||code;message=body.error?.message||message;}catch{}
  throw new ApiRequestError(response.status,code,message);
 }
 return response.status===204?undefined as T:response.json();
}

export {useMusicQueue,addMusicTracks} from './music-queue';
export type {QueuedTrack} from './music-queue';
