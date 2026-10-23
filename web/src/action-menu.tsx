import React,{useId,useRef,useState} from '@collector/runtime';

// Native popovers provide light dismissal, Escape and top-layer clipping safety.
export default function ActionMenu({label,children}:{label:string;children:React.ReactNode}){
 const id=useId();const trigger=useRef<HTMLButtonElement>(null);const popup=useRef<HTMLDivElement>(null);const [open,setOpen]=useState(false);
 return <span className="action-menu"><button ref={trigger} type="button" className="action-menu-trigger" popoverTarget={id} aria-label={label} aria-expanded={open} title={label}>⋯</button><div ref={popup} id={id} popover="auto" className="action-menu-popup" onToggle={event=>{
  const visible=event.newState==='open';setOpen(visible);if(!visible||!trigger.current||!popup.current)return;
  const anchor=trigger.current.getBoundingClientRect(),menu=popup.current;
  menu.style.left=`${Math.max(8,Math.min(anchor.right-menu.offsetWidth,innerWidth-menu.offsetWidth-8))}px`;
  menu.style.top=`${Math.max(8,Math.min(anchor.bottom+4,innerHeight-menu.offsetHeight-8))}px`;
 }} onClick={event=>{if((event.target as HTMLElement).closest('button'))popup.current?.hidePopover();}}>{children}</div></span>;
}
