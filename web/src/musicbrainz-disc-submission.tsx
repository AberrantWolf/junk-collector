import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';

type Coverage=components['schemas']['MusicAlbumCoverage'];
type Props={itemId:string;releaseId:string;refreshKey:string;suspended:boolean;onSessionExpired:()=>void};

// The application coverage response owns TOC eligibility and registration checks.
// Scope results to the selected release and revision so obsolete requests cannot
// leave an actionable link beside a different selection.
export default function MusicBrainzDiscSubmission({itemId,releaseId,refreshKey,suspended,onSessionExpired}:Props){
 const [retry,setRetry]=useState(0);
 const scope=JSON.stringify([itemId,releaseId,refreshKey,suspended,retry]);
 const [result,setResult]=useState<{scope:string;coverage?:Coverage;error?:string}>();
 const expired=useRef(onSessionExpired);expired.current=onSessionExpired;
 useEffect(()=>{
  if(suspended)return;
  const controller=new AbortController();let timer:ReturnType<typeof setTimeout>;
  async function load(){
   try{
    const coverage=await api<Coverage>(`/items/${itemId}/music-disc-coverage`,{signal:controller.signal});
    if(controller.signal.aborted)return;
    setResult({scope,coverage});timer=setTimeout(load,5000);
   }catch(error){
    if(controller.signal.aborted)return;
    setResult({scope,error:requestErrorMessage(error,'Disc ID submission is unavailable.')});
    if(isApiRequestError(error)&&error.code==='unauthenticated'){expired.current();return;}
    if(!isApiRequestError(error)||!['forbidden','unauthenticated'].includes(error.code)&&error.status!==404)timer=setTimeout(load,5000);
   }
  }
  void load();return()=>{controller.abort();clearTimeout(timer);};
 },[scope,itemId,suspended]);
 if(suspended||result?.scope!==scope)return null;
 if(result.error)return <div className="metadata-disc-submissions"><p role="alert">{result.error}</p><button onClick={()=>setRetry(value=>value+1)}>Retry Disc ID submission check</button></div>;
 const seen=new Set<string>();
 const sources=result.coverage?.release_id===releaseId?result.coverage.sources.filter(source=>{
  const toc=source.musicbrainz_submission_toc;if(!toc||seen.has(toc))return false;seen.add(toc);return true;
 }):[];
 if(!sources.length)return null;
 return <div className="metadata-disc-submissions">
  {sources.map((source,index)=><div key={source.source_representation_id}>
   {sources.length>1&&<small>{source.medium_position?`Disc ${source.medium_position}`:`Original ${index+1}`} · {source.track_count} tracks · {source.file_label}</small>}
   <a className="musicbrainz-submit" href={`https://musicbrainz.org/cdtoc/attach?${new URLSearchParams({toc:source.musicbrainz_submission_toc!,'filter-release.query':releaseId})}`} target="_blank" rel="noopener noreferrer">Submit Disc ID to MusicBrainz</a>
  </div>)}
  <small>Review and confirm on MusicBrainz. No audio is uploaded. Refresh this release after submitting.</small>
 </div>;
}
