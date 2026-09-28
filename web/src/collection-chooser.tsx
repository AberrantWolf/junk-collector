import React,{useEffect,useRef,useState,api,useOperationScope,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';

export type Collection=components['schemas']['PresentedCollection'];
type Page={items:Collection[];next_cursor:string|null};
type Filter={minRole?:0|1|2;module?:string};
type SearchState={key:string;items:Collection[];cursor:string|null;loading:boolean;error:string};

export function useCollectionSearch(value:string,filter:Filter={},initial?:Collection,onSessionExpired?:()=>void){
 const [search,setSearch]=useState('');const [state,setState]=useState<SearchState>({key:'',items:[],cursor:null,loading:true,error:''});const [selected,setSelected]=useState<Collection|undefined>(initial?.id===value?initial:undefined);const [revision,setRevision]=useState(0);
 const expired=useRef(onSessionExpired);expired.current=onSessionExpired;
 const minRole=filter.minRole, module=filter.module;
 const key=JSON.stringify([search,minRole,module,revision]);const reads=useOperationScope(key);
 const current=state.key===key;const items=current?state.items:[];const cursor=current?state.cursor:null;const loading=!current||state.loading;const error=current?state.error:'';
 function fail(error:unknown){if(isApiRequestError(error)&&error.code==='unauthenticated')expired.current?.();else setState(state=>state.key===key?{...state,error:requestErrorMessage(error)}:state);}
 useEffect(()=>{const online=()=>setRevision(value=>value+1);window.addEventListener('online',online);return()=>window.removeEventListener('online',online);},[]);
 function path(next?:string){const params=new URLSearchParams({limit:'50',search});if(next)params.set('cursor',next);if(minRole!==undefined)params.set('min_role',String(minRole));if(module)params.set('module',module);return `/collection-search?${params}`;}
 useEffect(()=>{setState({key,items:[],cursor:null,loading:true,error:''});const timer=setTimeout(()=>void reads.run(attempt=>api<Page>(path(),{signal:attempt.signal}),{success:page=>setState({key,items:page.items,cursor:page.next_cursor,loading:false,error:''}),failure:fail,settled:()=>setState(state=>state.key===key?{...state,loading:false}:state)}),150);return()=>clearTimeout(timer);},[key]);
 useEffect(()=>{if(!value){setSelected(undefined);return;}if(selected?.id===value)return;const known=items.find(item=>item.id===value)||initial?.id===value&&initial;if(known){setSelected(known);return;}const controller=new AbortController();api<Collection>(`/collections/${value}`,{signal:controller.signal}).then(item=>{if(!controller.signal.aborted)setSelected(item);}).catch(e=>{if(!controller.signal.aborted)fail(e);});return()=>controller.abort();},[value,items,initial]);
 async function more(){if(!current||!cursor||loading)return;setState(state=>state.key===key?{...state,loading:true,error:''}:state);await reads.run(attempt=>api<Page>(path(cursor),{signal:attempt.signal}),{success:page=>setState(state=>state.key===key?{...state,items:[...state.items,...page.items.filter(item=>!state.items.some(previous=>previous.id===item.id))],cursor:page.next_cursor}:state),failure:fail,settled:()=>setState(state=>state.key===key?{...state,loading:false}:state)});}
 return {search,setSearch,items,cursor,selected:selected?.id===value?selected:undefined,loading,error,more,retry:()=>setRevision(value=>value+1),choose:(item:Collection)=>setSelected(item)};
}

export function CollectionChooser({label,value,onChange,choices,disabled=false,all=false,selectable}:{label:string;value:string;onChange:(value:string)=>void;choices:ReturnType<typeof useCollectionSearch>;disabled?:boolean;all?:boolean;selectable?:(item:Collection)=>boolean}){
 const visible=choices.selected&&!choices.items.some(item=>item.id===choices.selected?.id)?[choices.selected,...choices.items]:choices.items;
 return <div className="collection-chooser"><label>{label}<input type="search" maxLength={256} value={choices.search} onChange={event=>choices.setSearch(event.target.value)} placeholder="Find a collection" disabled={disabled}/></label><div className="destination-results">{all&&<button type="button" disabled={disabled} aria-pressed={!value} onClick={()=>onChange('')}>All collections</button>}{visible.map(item=><button type="button" key={item.id} disabled={disabled||selectable?.(item)===false} aria-pressed={value===item.id} onClick={()=>{choices.choose(item);onChange(item.id);}}><strong>{item.name}</strong><small>{item.id===choices.selected?.id&&!choices.items.some(row=>row.id===item.id)?'Selected · ':''}{item.binding.state==='assigned'?item.binding.module:'Legacy collection'} · {item.id.slice(0,8)}{selectable?.(item)===false?' · Content unavailable':''}</small></button>)}</div>{choices.loading&&<p role="status">Loading collections…</p>}{choices.error&&<p role="alert" className="notice error">{choices.error} <button type="button" onClick={choices.retry}>Retry collections</button></p>}{!choices.loading&&!choices.error&&!visible.length&&<p className="muted">No matching collections.</p>}{choices.cursor&&<button type="button" disabled={disabled||choices.loading} onClick={()=>void choices.more()}>More collections</button>}</div>;
}
