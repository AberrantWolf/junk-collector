import React,{useEffect,useState,usePlaybackPreference,setPlaybackPreference,validOpusBitrate,type PlaybackFormat} from '@collector/runtime';

export default function PlaybackFormatControl({compact=false}:{compact?:boolean}){
 const preference=usePlaybackPreference();
 const [bitrate,setBitrate]=useState(String(preference.bitrate));
 useEffect(()=>setBitrate(String(preference.bitrate)),[preference.bitrate]);
 return <form className={compact?'playback-format dock-format':'playback-format'} onSubmit={e=>{
  e.preventDefault();const rate=Number(bitrate);
  if(validOpusBitrate(rate))setPlaybackPreference({...preference,bitrate:rate});
 }}>
  <label>Playback format<select value={preference.format} onChange={e=>setPlaybackPreference({...preference,format:e.target.value as PlaybackFormat})}>
   <option value="original">Original</option><option value="high_bandwidth">High Bandwidth</option><option value="low_bandwidth">Low Bandwidth</option>
  </select></label>
  {preference.format==='low_bandwidth'&&<><label>Opus bitrate (kbps)<input type="number" required min={6} max={256} step={1} value={bitrate} onChange={e=>setBitrate(e.target.value)}/></label><button type="submit" disabled={!validOpusBitrate(Number(bitrate))||Number(bitrate)===preference.bitrate}>Apply bitrate</button></>}
  {!compact&&<p className="muted">Saved for this account in this browser. Changes apply the next time you press Play or start a track. Live conversion uses the normal Collector audio connection.</p>}
 </form>;
}
