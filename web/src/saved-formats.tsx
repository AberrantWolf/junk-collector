import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage,retainUnchanged} from '@collector/runtime';
import type {components} from './generated/api';
import {profileLabel} from './audio-profiles';
import {bytes} from './work-progress';
type S=components['schemas'];
type Album=S['SavedFormatAlbum'];
type Format=S['SavedFormatSummary'];
type PageView=S['SavedFormatsPage'];
const origins:Record<S['FormatRequest'],string>={track:'one track',album:'an album',batch:'a bulk selection',import_setting:'the import setting'};
const key=(profile:Format['profile'])=>profile.codec==='flac'?'flac':`opus:${profile.bitrate_kbps}`;

// Saved formats are extra copies (FLAC, Opus) kept beside the originals. This
// view lists them, shows what they cost, and removes what isn't wanted.
// Originals are never listed here and can't be removed from here.
export default function SavedFormats({collection,search,onOpenAlbum,onAddFormat,onSessionExpired}:{collection:string;search:string;onOpenAlbum:(item:string)=>void;onAddFormat:()=>void;onSessionExpired:()=>void}){
 const [format,setFormat]=useState(''),[origin,setOrigin]=useState(''),[partial,setPartial]=useState(false),[severalOpus,setSeveralOpus]=useState(false);
 const [items,setItems]=useState<Album[]>([]),[totals,setTotals]=useState<PageView['totals']>([]),[cursor,setCursor]=useState<string|null>(null),[pages,setPages]=useState(1),[loading,setLoading]=useState(true),[error,setError]=useState(''),[busy,setBusy]=useState(false),[version,setVersion]=useState(0),[notice,setNotice]=useState('');
 const [selected,setSelected]=useState<string[]>([]),[bulkFormat,setBulkFormat]=useState('');
 const sentinel=useRef<HTMLDivElement>(null);
 function fail(e:unknown){if(isApiRequestError(e)&&e.code==='unauthenticated')onSessionExpired();else setError(requestErrorMessage(e));}
 const scope=JSON.stringify([collection,search,format,origin,partial,severalOpus,version]);
 useEffect(()=>{setPages(1);setSelected([]);},[collection,search,format,origin,partial,severalOpus]);
 useEffect(()=>{const controller=new AbortController();setLoading(true);setError('');
  (async()=>{try{const rows:Album[]=[];let next:string|null=null;let sums:PageView['totals']=[];for(let n=0;n<pages;n++){const q=new URLSearchParams({limit:'50'});if(collection)q.set('collection_id',collection);if(search)q.set('search',search);if(format)q.set('format',format);if(origin)q.set('requested_by',origin);if(partial)q.set('partial_only','true');if(severalOpus)q.set('several_opus','true');if(next)q.set('cursor',next);const page=await api<PageView>(`/saved-formats?${q}`,{signal:controller.signal});rows.push(...page.items);if(!n)sums=page.totals;next=page.next_cursor??null;if(!next)break;}
   if(controller.signal.aborted)return;setItems(current=>retainUnchanged(current,rows));setTotals(current=>retainUnchanged(current,sums));setCursor(next);}catch(e){if(!controller.signal.aborted)fail(e);}finally{if(!controller.signal.aborted)setLoading(false);}})();
  return()=>controller.abort();},[scope,pages]);
 useEffect(()=>{const element=sentinel.current;if(!element||!cursor||loading)return;const observer=new IntersectionObserver(entries=>{if(entries.some(entry=>entry.isIntersecting))setPages(value=>value+1);},{rootMargin:'400px'});observer.observe(element);return()=>observer.disconnect();},[cursor,loading]);
 async function remove(albums:Album[],profileKey:string){
  const targets=albums.flatMap(album=>album.formats.filter(f=>key(f.profile)===profileKey).map(f=>({album,format:f})));
  if(!targets.length)return;
  const size=targets.reduce((sum,{format})=>sum+format.bytes,0);
  const label=profileLabel(targets[0].format.profile);
  if(!window.confirm(`Remove the saved ${label} copy from ${targets.length===1?targets[0].album.title:`${targets.length} albums`} (${bytes(size)})?\n\nThe original files stay. Those tracks will play from the originals through live conversion, and the copy can be made again later. The removed copy stays in the archive's history, so this doesn't free disk space yet.`))return;
  setBusy(true);setError('');setNotice('');let removed=0;
  try{for(const {album,format} of targets){for(const capture of format.captures){await api(`/items/${album.item_id}/saved-formats/remove`,{method:'POST',headers:{'Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({capture_id:capture.capture_id,expected_revision:capture.revision,profile:format.profile})});}removed++;}
   setNotice(`Removed the ${label} copy from ${removed} ${removed===1?'album':'albums'}.`);}
  catch(e){fail(e);if(removed)setNotice(`Removed the ${label} copy from ${removed} ${removed===1?'album':'albums'} before the error.`);}
  finally{setBusy(false);setSelected([]);setVersion(v=>v+1);}
 }
 const formatOptions=totals.map(total=>({value:key(total.profile),label:profileLabel(total.profile)}));
 const chosen=items.filter(album=>selected.includes(album.item_id));
 const chosenFormats=[...new Map(chosen.flatMap(album=>album.formats).map(f=>[key(f.profile),profileLabel(f.profile)])).entries()];
 return <section className="saved-formats" aria-label="Saved formats">
  <p className="muted">Every supported original plays without a saved copy: the server converts it while it streams. Saved copies are extras, made because someone asked for them.</p>
  {totals.length>0&&<ul className="saved-format-totals" aria-label="Space used by saved formats">{totals.map(total=><li key={key(total.profile)}><strong>{profileLabel(total.profile)}</strong> {bytes(total.bytes)} · {total.albums} {total.albums===1?'album':'albums'}</li>)}</ul>}
  <div className="care-filters">
   <label>Format<select value={format} onChange={e=>setFormat(e.target.value)}><option value="">Any saved format</option>{formatOptions.map(option=><option key={option.value} value={option.value}>{option.label}</option>)}</select></label>
   <label>Made for<select value={origin} onChange={e=>setOrigin(e.target.value)}><option value="">Anything</option>{(Object.keys(origins) as S['FormatRequest'][]).map(value=><option key={value} value={value}>{origins[value]}</option>)}</select></label>
   <label className="inline-check"><input type="checkbox" checked={partial} onChange={e=>setPartial(e.target.checked)}/> Only partly made</label>
   <label className="inline-check"><input type="checkbox" checked={severalOpus} onChange={e=>setSeveralOpus(e.target.checked)}/> Several Opus bitrates</label>
   <button onClick={onAddFormat}>Add a format…</button>
  </div>
  {error&&<p role="alert" className="notice error">{error}</p>}
  {notice&&<p role="status" className="notice">{notice}</p>}
  {selected.length>0&&<div className="care-selection" role="region" aria-label="Selected albums"><strong>{selected.length} selected</strong><label>Remove format<select value={bulkFormat} onChange={e=>setBulkFormat(e.target.value)}><option value="">Choose…</option>{chosenFormats.map(([value,label])=><option key={value} value={value}>{label}</option>)}</select></label><button disabled={busy||!bulkFormat} onClick={()=>void remove(chosen,bulkFormat)}>Remove from selected</button><button onClick={()=>setSelected([])}>Clear selection</button></div>}
  <div className="care-rows" aria-busy={loading}>{items.map(album=><div className="review-row saved-format-row" key={album.item_id}>
   <label className="review-select"><input type="checkbox" aria-label={`Select ${album.title}`} checked={selected.includes(album.item_id)} onChange={e=>setSelected(current=>e.target.checked?[...current,album.item_id]:current.filter(id=>id!==album.item_id))}/></label>
   <button className="review-row-main" onClick={()=>onOpenAlbum(album.item_id)}>{album.artwork?<img loading="lazy" src={`/api/v1/collections/${album.collection_id}/assets/${album.artwork.id}/thumbnail?revision=${album.artwork.revision}`} alt=""/>:<span className="care-cover-empty" aria-hidden="true">♪</span>}<span className="row-text"><strong>{album.title}</strong><small>{album.subtitle}</small></span></button>
   <div className="saved-format-list">{album.formats.map(f=>{const incomplete=album.album_tracks>0&&f.tracks<album.album_tracks;return <div className={`saved-format ${incomplete?'partial':''}`} key={key(f.profile)}>
    <span><strong>{profileLabel(f.profile)}</strong> {incomplete?`⚠ ${f.tracks} of ${album.album_tracks} tracks`:'✓'} · {bytes(f.bytes)}</span>
    <small>{f.requested_by.length?`Made for ${f.requested_by.map(value=>origins[value]).join(', ')}`:''}{f.unknown_origin?`${f.requested_by.length?' · ':''}made before this was recorded`:''}</small>
    {incomplete&&<small>The other tracks play from the originals. Open the album to see why they weren't made, finish them, or remove this copy.</small>}
    <span className="button-row">{incomplete&&<button onClick={()=>onOpenAlbum(album.item_id)}>Fix</button>}<button disabled={busy} onClick={()=>void remove([album],key(f.profile))}>Remove</button></span>
   </div>;})}</div>
  </div>)}</div>
  {!items.length&&!loading&&!error&&<p className="review-empty">{format||origin||partial||severalOpus||search?'No albums match these filters.':'No saved formats yet. Originals play as they are.'}</p>}
  <div ref={sentinel} className="auto-more">{cursor?<button disabled={loading} onClick={()=>setPages(v=>v+1)}>{loading?'Loading more albums…':'Show more albums'}</button>:items.length>0&&<small className="muted">All {items.length} albums shown</small>}</div>
 </section>;
}
