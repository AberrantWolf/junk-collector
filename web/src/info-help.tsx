import React,{useEffect,useRef,useState,useId} from '@collector/runtime';

/** Short optional explanations; longer diagnostics belong in details elements. */
export default function InfoHelp({label,children}:{label:string;children:React.ReactNode}){
 const id=useId();
 const [open,setOpen]=useState(false);
 const root=useRef<HTMLSpanElement>(null);
 useEffect(()=>{
  if(!open)return;
  function dismiss(event:PointerEvent){if(!root.current?.contains(event.target as Node))setOpen(false);}
  document.addEventListener('pointerdown',dismiss);
  return()=>document.removeEventListener('pointerdown',dismiss);
 },[open]);
 return <span ref={root} className="info-help" onBlur={event=>{if(!event.currentTarget.contains(event.relatedTarget as Node|null))setOpen(false);}} onKeyDown={event=>{if(event.key==='Escape'&&open){event.preventDefault();event.stopPropagation();setOpen(false);}}}>
  <button type="button" className="info-help-button" aria-label={`About ${label}`} aria-expanded={open} aria-controls={id} onClick={()=>setOpen(value=>!value)}>i</button>
  <span id={id} hidden={!open} className="info-help-content" role="note">{children}</span>
 </span>;
}
