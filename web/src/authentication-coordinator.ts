export type AuthenticationIdentity={user_id:string;device_id?:string|null};
export type AuthenticationHandler=(request:{identity:AuthenticationIdentity;signal:AbortSignal})=>Promise<AuthenticationIdentity>;
type Waiter={resolve:(identity:AuthenticationIdentity)=>void;reject:(error:unknown)=>void;detach:()=>void};
/** Shares the prompt, not command ownership. Each waiting command keeps its own lifetime. */
export class AuthenticationCoordinator {
 private pending?:{identity:AuthenticationIdentity;controller:AbortController;waiters:Set<Waiter>};
 constructor(private handler:AuthenticationHandler){}
 cancel(){const pending=this.pending;if(!pending)return;this.pending=undefined;pending.controller.abort();for(const waiter of pending.waiters){waiter.detach();waiter.reject(new DOMException('Authentication cancelled','AbortError'));}pending.waiters.clear();}
 wait(identity:AuthenticationIdentity,signal?:AbortSignal):Promise<AuthenticationIdentity>{
  signal?.throwIfAborted();
  if(this.pending&&this.pending.identity.user_id!==identity.user_id)return Promise.reject(new Error('Account changed. Review and submit the action again.'));
  let pending=this.pending;const start=!pending;
  if(!pending){pending={identity:{...identity},controller:new AbortController(),waiters:new Set()};this.pending=pending;}
  const current=pending;
  const result=new Promise<AuthenticationIdentity>((resolve,reject)=>{
   const abort=()=>{current.waiters.delete(waiter);waiter.detach();reject(new DOMException('Operation cancelled','AbortError'));if(!current.waiters.size&&this.pending===current){this.pending=undefined;current.controller.abort();}};
   const waiter:Waiter={resolve,reject,detach:()=>signal?.removeEventListener('abort',abort)};
   current.waiters.add(waiter);signal?.addEventListener('abort',abort,{once:true});
  });
  if(start){
   void Promise.resolve().then(()=>{current.controller.signal.throwIfAborted();return this.handler({identity:current.identity,signal:current.controller.signal});}).then(identity=>{
    if(identity.user_id!==current.identity.user_id)throw new Error('Account changed. The pending action was not submitted.');
    this.settle(current,identity);
   }).catch(error=>this.settle(current,undefined,error));
  }
  return result;
 }
 private settle(current:NonNullable<AuthenticationCoordinator['pending']>,identity?:AuthenticationIdentity,error?:unknown){
  if(this.pending!==current)return;this.pending=undefined;
  for(const waiter of current.waiters){waiter.detach();if(identity)waiter.resolve(identity);else waiter.reject(error);}current.waiters.clear();
 }
}
