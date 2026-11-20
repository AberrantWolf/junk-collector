import React,{useEffect,useState,usePlaybackPreference,setPlaybackPreference,validOpusBitrate,type PlaybackFormat} from '@collector/runtime';

export default function PlaybackFormatControl({compact=false}:{compact?:boolean}){
 const preference=usePlaybackPreference();
 const [bitrate,setBitrate]=useState(String(preference.bitrate));
 useEffect(()=>setBitrate(String(preference.bitrate)),[preference.bitrate]);
 const bitrateFields=<>
  <label>Opus bitrate (kbps)<input type="number" required min={6} max={256} step={1} value={bitrate} onChange={e=>setBitrate(e.target.value)}/></label>
  <button type="submit" aria-label="Apply bitrate" title="Apply bitrate" disabled={!validOpusBitrate(Number(bitrate))||Number(bitrate)===preference.bitrate}>
   {compact?<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true"><path d="m5 12 4 4L19 6"/></svg>:'Apply bitrate'}
  </button>
 </>;
 return <form className={compact?'playback-format dock-format':'playback-format'} onSubmit={e=>{
  e.preventDefault();const rate=Number(bitrate);
  if(validOpusBitrate(rate))setPlaybackPreference({...preference,bitrate:rate});
 }}>
  <label>Playback format<select value={preference.format} onChange={e=>setPlaybackPreference({...preference,format:e.target.value as PlaybackFormat})}>
   <option value="original">Original</option><option value="high_bandwidth">High Bandwidth</option><option value="low_bandwidth">Low Bandwidth</option>
  </select></label>
  {preference.format==='low_bandwidth'&&(compact?<details className="dock-bitrate" onKeyDown={event=>{
   if(event.key!=='Escape'||!event.currentTarget.open)return;
   event.stopPropagation();
   event.currentTarget.open=false;
   event.currentTarget.querySelector('summary')?.focus();
  }}>
   <summary aria-label="Opus bitrate settings" title="Opus bitrate settings"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true"><path d="M4 8h16M4 16h16M9 5v6M15 13v6"/></svg></summary>
   <div className="dock-bitrate-panel">{bitrateFields}</div>
  </details>:bitrateFields)}
 </form>;
}
