import {spawn as nodeSpawn,spawnSync} from 'node:child_process';
import path from 'node:path';
export {spawnSync};
export const root=path.resolve(import.meta.dirname,'../../..');
export const serverBinary=process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server');
export const mediaBinary=process.env.COLLECTOR_MEDIA_BINARY||path.join(root,'target/debug/collector-media');
const children=new Set();
const cleanups=new Set();
export async function deadline(promise,label,millis=10000){
 let timer;try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(`${label} timed out after ${millis}ms`)),millis);})]);}finally{clearTimeout(timer);}
}
export async function stopProcess(child,millis=5000){
 if(!child||child.exitCode!==null||child.signalCode!==null)return;
 const ended=new Promise(resolve=>child.once('exit',resolve));
 child.kill('SIGTERM');
 try{await deadline(ended,'child shutdown',millis);}catch{child.kill('SIGKILL');await deadline(ended,'child reap',5000);}
}
export function spawn(...args){
 const child=nodeSpawn(...args);children.add(child);child.once('exit',()=>children.delete(child));return child;
}
export function onCleanup(callback){cleanups.add(callback);return ()=>cleanups.delete(callback);}
export async function cleanup(){
 const results=await Promise.allSettled([...cleanups].map(callback=>callback()));
 const stopped=await Promise.allSettled([...children].map(child=>stopProcess(child)));
 const failed=[...results,...stopped].find(result=>result.status==='rejected');
 if(failed)throw failed.reason;
}
export async function waitForServer(url,child,{timeout=20000,logs=()=>''}={}){
 const until=Date.now()+timeout;
 while(Date.now()<until){
  if(child&&(child.exitCode!==null||child.signalCode!==null))throw Error(`Server exited before readiness: ${logs()}`);
  try{const response=await fetch(url,{signal:AbortSignal.timeout(1000)});if(response.ok)return;}catch{}
  await new Promise(resolve=>setTimeout(resolve,50));
 }
 throw Error(`Server readiness timed out: ${url}\n${logs()}`);
}
let exiting=false;
async function finish(code,error){
 if(exiting)return;exiting=true;if(error)console.error(error);
 try{await cleanup();}catch(failure){console.error(failure);code=1;}
 process.exit(code);
}
process.once('SIGINT',()=>void finish(130));
process.once('SIGTERM',()=>void finish(143));
process.once('uncaughtException',error=>void finish(1,error));
process.once('unhandledRejection',error=>void finish(1,error));
