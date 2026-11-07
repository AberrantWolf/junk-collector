import React from '@collector/runtime';

export default function MoreButton({children,loading,loadingText,disabled,className,onClick}:{
 children:string;loading:boolean;loadingText:string;disabled?:boolean;className?:string;onClick:()=>void;
}){
 return <button type="button" className={className} disabled={loading||disabled} onClick={onClick}>
  {loading?<><span className="page-spinner" aria-hidden="true"/>{loadingText}</>:children}
 </button>;
}
