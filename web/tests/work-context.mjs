import {build} from 'esbuild';
import assert from 'node:assert/strict';
import path from 'node:path';
async function load(name){const result=await build({entryPoints:[path.resolve(import.meta.dirname,`../src/${name}.ts`)],bundle:true,write:false,platform:'node',format:'esm',define:{'process.env.NODE_ENV':'"production"'}});return import('data:text/javascript;base64,'+Buffer.from(result.outputFiles[0].text).toString('base64'));}
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
const {OperationScope}=await load('operation-scope');
const scope=new OperationScope('A');const old=deferred();const callbacks=[];
const first=scope.run(()=>old.promise,{success:()=>callbacks.push('old success'),failure:()=>callbacks.push('old error'),settled:()=>callbacks.push('old cleanup')});
scope.bind('B');scope.bind('A');await scope.run(async()=>42,{success:value=>callbacks.push(value),settled:()=>callbacks.push('new cleanup')});old.reject(new Error('late failure'));await first;assert.deepEqual(callbacks,[42,'new cleanup']);
const ignoredAbort=deferred();const detached=scope.run(()=>ignoredAbort.promise,{success:()=>callbacks.push('detached')});scope.cancel();ignoredAbort.resolve(1);await detached;assert.equal(callbacks.includes('detached'),false);
scope.dispose();const disposedCallbacks=[];await scope.run(async()=>{disposedCallbacks.push(0);return 1;},{success:()=>disposedCallbacks.push(1)});assert.deepEqual(disposedCallbacks,[]);
const sibling=new OperationScope('A');const alive=sibling.begin();scope.cancel();assert.equal(alive.current(),true);
const {AuthenticationCoordinator}=await load('authentication-coordinator');const alice={user_id:'alice',device_id:'old'};
const prompts=[];const coordinator=new AuthenticationCoordinator(request=>{const gate=deferred();prompts.push({request,gate});return gate.promise;});
const a=new AbortController(),b=new AbortController();const cancelled=coordinator.wait(alice,a.signal);const cancelledCheck=assert.rejects(cancelled,{name:'AbortError'});const survives=coordinator.wait(alice,b.signal);await Promise.resolve();assert.equal(prompts.length,1);a.abort();await cancelledCheck;assert.equal(prompts[0].request.signal.aborted,false);prompts[0].gate.resolve({...alice,device_id:'new'});assert.equal((await survives).user_id,'alice');
const c=new AbortController();const abandoned=coordinator.wait(alice,c.signal);const abandonedCheck=assert.rejects(abandoned,{name:'AbortError'});await Promise.resolve();c.abort();await abandonedCheck;assert.equal(prompts[1].request.signal.aborted,true);
const latest=coordinator.wait(alice);await Promise.resolve();let finished=false;latest.then(()=>finished=true);prompts[1].gate.resolve(alice);await Promise.resolve();await Promise.resolve();assert.equal(finished,false);prompts[2].gate.resolve(alice);await latest;
const wrong=coordinator.wait(alice);await Promise.resolve();prompts[3].gate.resolve({user_id:'bob'});await assert.rejects(wrong,/Account changed/);
// The shared browser runtime emits identity-change events even in this Node test.
globalThis.window=new EventTarget();
const runtime=await load('runtime');runtime.setApiIdentity(alice);
const prompt=deferred(),promptStarted=deferred();runtime.setRecentAuthenticationHandler(async()=>{promptStarted.resolve();return prompt.promise;});
const calls=[];globalThis.fetch=async(url,init)=>{calls.push({url,body:init.body,key:new Headers(init.headers).get('Idempotency-Key'),actor:new Headers(init.headers).get('X-Collector-Actor')});if(url==='/api/v1/me')return Response.json({...alice,device_id:'new'});if(calls.filter(call=>call.url===url).length===1)return Response.json({error:{code:'recent_authentication_required'}},{status:403});return Response.json({saved:true});};
const init={method:'PUT',body:JSON.stringify({target:'A',revision:3}),headers:{'Idempotency-Key':'immutable-command'}};const command=runtime.api('/collections/A/grants/alice',init);await promptStarted.promise;init.body=JSON.stringify({target:'B'});init.headers['Idempotency-Key']='wrong-key';prompt.resolve({...alice,device_id:'new'});await command;const writes=calls.filter(call=>call.url.includes('grants'));assert.equal(writes.length,2);assert.equal(writes[0].actor,'alice');assert.deepEqual(writes[1],writes[0]);
runtime.setRecentAuthenticationHandler(async()=>({...alice,device_id:'new'}));await assert.rejects(runtime.api('/devices/old',{method:'DELETE',authenticationPolicy:'review'}),error=>error.code==='authentication_review_required');assert.equal(calls.filter(call=>call.url==='/api/v1/devices/old').length,1);
runtime.setRecentAuthenticationHandler(async()=>({user_id:'bob'}));await assert.rejects(runtime.api('/other-command',{method:'POST'}),/Account changed/);assert.equal(calls.filter(call=>call.url==='/api/v1/other-command').length,1);
runtime.setRecentAuthenticationHandler();
console.log('Work context passed: stale success/error/cleanup, A-B-A, detach, independent lanes, independent auth waiters, last-waiter cancellation, obsolete prompts, account identity, immutable command retry and session review.');
