import React from '@collector/runtime';
import type {UploadProgress} from './package-upload';
export function bytes(value:number){return value<1024?`${value} B`:value<1048576?`${(value/1024).toLocaleString(undefined,{maximumFractionDigits:1})} KiB`:`${(value/1048576).toLocaleString(undefined,{maximumFractionDigits:1})} MiB`;}
export function WorkProgress({label,detail,completed,total}:{label:string;detail?:string;completed?:number;total?:number}){
 const known=completed!==undefined&&total!==undefined&&total>0;
 return <div className="work-progress"><div><strong>{label}</strong>{known&&<span>{Math.floor(Math.min(completed,total)/total*100)}%</span>}</div><progress aria-label={label} max={known?total:undefined} value={known?Math.min(completed,total):undefined}/>{detail&&<p role="status">{detail}</p>}</div>;
}
export function UploadMeter({value}:{value:UploadProgress}){
 return <WorkProgress label={value.phase==='checking'?'Checking uploaded files':'Uploading files'} completed={value.waiting?undefined:value.completed} total={value.total} detail={`${value.file} · file ${value.index} of ${value.count} · ${bytes(value.completed)} / ${bytes(value.total)} ${value.phase==='checking'?'sent for checking':'sent'} · ${bytes(value.confirmed)} / ${bytes(value.packageBytes)} saved on server. ${value.waiting?'Waiting for the server to save these files.':'Keep this window open until uploading finishes.'}`}/>;
}
