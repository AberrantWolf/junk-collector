/** A UI lane: replacing an attempt invalidates its callbacks even if I/O ignores abort. */
export class OperationScope {
 private disposed=false;
 private controller?:AbortController;
 private key:string;
 constructor(key=''){this.key=key;}
 bind(key:string){if(key!==this.key){this.cancel();this.key=key;}}
 activate(){this.disposed=false;}
 dispose(){this.disposed=true;this.cancel();}
 cancel(){this.controller?.abort();this.controller=undefined;}
 begin(){
  this.cancel();const controller=new AbortController();this.controller=controller;if(this.disposed)controller.abort();
  const current=()=>this.controller===controller&&!controller.signal.aborted;
  return {
   signal:controller.signal,
   current,
   check(){if(!current())throw new DOMException('Operation superseded','AbortError');},
   commit(effect:()=>void){if(current())effect();},
   async wait<T>(promise:Promise<T>):Promise<T>{const value=await promise;if(!current())throw new DOMException('Operation superseded','AbortError');return value;},
  };
 }
 async run<T>(work:(attempt:OperationAttempt)=>Promise<T>,callbacks:{success?:(value:T)=>void;failure?:(error:unknown)=>void;settled?:()=>void}={}){
  const attempt=this.begin();
  try{attempt.check();const value=await work(attempt);attempt.commit(()=>callbacks.success?.(value));}
  catch(error){attempt.commit(()=>callbacks.failure?.(error));}
  finally{attempt.commit(()=>callbacks.settled?.());}
 }
}
export type OperationAttempt=ReturnType<OperationScope['begin']>;
