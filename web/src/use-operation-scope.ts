import {useLayoutEffect,useMemo} from 'react';
import {OperationScope} from './operation-scope';

/** Invalidate the old lane at commit, without aborting requests during React render. */
export function useOperationScope(key:string){
 const scope=useMemo(()=>new OperationScope(key),[key]);
 useLayoutEffect(()=>{scope.activate();return()=>scope.dispose();},[scope]);
 return scope;
}
