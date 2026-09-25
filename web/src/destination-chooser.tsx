import React,{useEffect,useRef,useState,api,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type Page=components['schemas']['Page_PresentedItem'];

export default function DestinationChooser({collectionId,selected,onSelect,disabled=false,emptyMode='unattached'}:{collectionId:string;selected:Item|null;onSelect:(item:Item|null)=>void;disabled?:boolean;emptyMode?:'create'|'unattached'}){
 const [open,setOpen]=useState(false);const [query,setQuery]=useState('');const [rows,setRows]=useState<Item[]>([]);
 const [cursor,setCursor]=useState<string|null>(null);const [loading,setLoading]=useState(false);const [error,setError]=useState('');const [revision,setRevision]=useState(0);
 const moreController=useRef<AbortController|null>(null);
 useEffect(()=>()=>{moreController.current?.abort();},[query,collectionId,open]);
 useEffect(()=>{if(!open||!collectionId)return;const controller=new AbortController();const timer=setTimeout(async()=>{
  setLoading(true);setError('');try{const params=new URLSearchParams({collection_id:collectionId,search:query,limit:'50'});const page=await api<Page>(`/items?${params}`,{signal:controller.signal});if(!controller.signal.aborted){setRows(page.items);setCursor(page.next_cursor??null);}}catch(e){if(!controller.signal.aborted)setError(requestErrorMessage(e));}finally{if(!controller.signal.aborted)setLoading(false);}
 },180);return()=>{controller.abort();clearTimeout(timer);};},[open,collectionId,query,revision]);
 async function more(){if(!cursor||loading)return;moreController.current?.abort();const controller=new AbortController();moreController.current=controller;setLoading(true);setError('');try{const params=new URLSearchParams({collection_id:collectionId,search:query,limit:'50',cursor});const page=await api<Page>(`/items?${params}`,{signal:controller.signal});if(!controller.signal.aborted){setRows(current=>[...current,...page.items.filter(item=>!current.some(previous=>previous.id===item.id))]);setCursor(page.next_cursor??null);}}catch(e){if(!controller.signal.aborted)setError(requestErrorMessage(e));}finally{if(!controller.signal.aborted)setLoading(false);}}
 return <section className="destination-chooser" aria-label="Destination entry">
  <p><strong>Destination</strong> · {selected?`${selected.title}${selected.subtitle?` · ${selected.subtitle}`:''}`:emptyMode==='create'?'Create a new entry for the package':'Leave unattached'}</p>
  <p className="muted">No evidence-backed destination suggestions are available. Search the collection to choose an entry; a similar title alone is not a detected match.</p>
  {selected&&<button type="button" disabled={disabled} onClick={()=>onSelect(null)}>{emptyMode==='create'?'Create a new entry instead':'Leave unattached'}</button>}
  <button type="button" disabled={disabled} aria-expanded={open} onClick={()=>setOpen(value=>!value)}>{open?'Close search':'Search existing entries'}</button>
  {open&&<><label>Search existing entries<input type="search" value={query} onChange={event=>{setQuery(event.target.value);setRows([]);setCursor(null);}} placeholder="Title, artist or edition"/></label>
   {loading&&!rows.length&&<p role="status">Searching entries…</p>}{error&&<p role="alert" className="notice error">{error} <button type="button" onClick={()=>setRevision(value=>value+1)}>Retry</button></p>}
   {!loading&&!error&&!rows.length&&<p>No matching entries. Try another search.</p>}
   <div className="destination-results">{rows.map(item=>{const artwork=item.assets.find(asset=>asset.id===item.primary_artwork);const context=item.presentation.fields.filter(field=>field.label!=='Collection type').slice(0,2).map(field=>field.value).join(' · ');return <button type="button" key={item.id} disabled={disabled} aria-pressed={selected?.id===item.id} onClick={()=>{onSelect(item);setOpen(false);}}>{artwork?<img src={`/api/v1/collections/${item.collection_id}/assets/${artwork.id}/thumbnail`} alt={artwork.alt_text||`${item.title} artwork`} loading="lazy"/>:<span className="destination-placeholder" aria-label="No artwork">◇</span>}<span><strong>{item.title}</strong><small>{[item.subtitle,context].filter(Boolean).join(' · ')||'No edition details'}</small></span></button>;})}</div>
   {cursor&&<button type="button" disabled={loading} onClick={()=>void more()}>{loading?'Loading…':'More results'}</button>}
  </>}
 </section>;
}
