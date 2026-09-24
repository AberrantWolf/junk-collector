import {useEffect,useRef} from '@collector/runtime';

// Each editor owns its draft state. Dismissal asks about all drafts in the
// dialog, including nested forms, without treating saved mutations as drafts.
export function useDialogDismiss(onClose:()=>void){
 const ref=useRef<HTMLElement>(null);
 const close=useRef(onClose);close.current=onClose;
 function dismiss(){
  const element=ref.current;if(!element)return false;
  if(element.querySelector('[data-uploading="true"]')){
   if(!window.confirm('Pause this upload and close? Saved entries and received bytes are kept. Reopen Import dumps and select the same folder or files to resume.'))return false;
  }else if(element.matches('[data-unsaved="true"]')||element.querySelector('[data-unsaved="true"]')){
   if(!window.confirm('Discard unsaved changes and close? Saved records and uploaded files will be kept.'))return false;
  }else if(element.getAttribute('aria-busy')==='true'&&!window.confirm('Work is still in progress. Close this window? Submitted work may continue on the server.'))return false;
  close.current();return true;
 }
 const latest=useRef(dismiss);latest.current=dismiss;
 useEffect(()=>{
  const previous=document.activeElement as HTMLElement|null;
  const element=ref.current;const overflow=document.body.style.overflow;
  document.body.style.overflow='hidden';element?.querySelector<HTMLButtonElement>('.close')?.focus();
  function keydown(event:KeyboardEvent){
   if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();latest.current();}
   if(event.key==='Tab'&&element){
    const controls=[...element.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]')].filter(value=>value.getClientRects().length>0);
    const first=controls[0],last=controls.at(-1);
    if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}
    else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus();}
   }
  }
  document.addEventListener('keydown',keydown,true);
  return()=>{document.removeEventListener('keydown',keydown,true);document.body.style.overflow=overflow;if(previous?.isConnected)previous.focus();};
 },[]);
 return {ref,dismiss};
}
