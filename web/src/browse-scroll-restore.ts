import {useEffect} from '@collector/runtime';

export type ScrollTarget={scroll:number;focus?:string};
export type BrowseRestore={current:ScrollTarget|null};

export function useBrowseScrollRestore(restore:BrowseRestore, ready:boolean, revision:number, focusRow:boolean){
 useEffect(()=>{
  const target=restore.current;
  if(!ready||!target)return;
  const frame=requestAnimationFrame(()=>{
   if(restore.current!==target)return;
   if(focusRow&&target.focus){
    document.querySelector<HTMLElement>(`[data-item-id="${CSS.escape(target.focus)}"]`)?.focus({preventScroll:true});
   }
   window.scrollTo(0,target.scroll);
   restore.current=null;
  });
  return()=>cancelAnimationFrame(frame);
 },[restore,ready,revision,focusRow]);
}
