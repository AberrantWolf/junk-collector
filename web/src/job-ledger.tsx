import React,{useEffect,useRef,useState,api,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';

type Job=components['schemas']['JobView'];
type Page=components['schemas']['Page_JobView'];
const activeStates=new Set<Job['state']>(['queued','running','cancel_requested']);
const kindLabels:Record<string,string>={integrity_verification:'Integrity check',preservation_import:'Legacy capture publication',import_staging:'Package staging',import_analysis:'Import analysis',import_publication:'Capture publication'};

function percentage(job:Job){return job.total_units?Math.min(100,Math.round(job.completed_units/job.total_units*100)):job.state==='completed'?100:0;}
function readable(value:string){return value.replaceAll('_',' ');}

export default function JobLedger(){
 const [jobs,setJobs]=useState<Job[]>([]);const [cursor,setCursor]=useState<string|null>();const [error,setError]=useState('');const [changing,setChanging]=useState('');
 const timer=useRef<ReturnType<typeof setTimeout>|undefined>(undefined);const live=useRef(true);

 async function fetchJobs(append=false){
  try{
   const query=new URLSearchParams({limit:'100'});if(append&&cursor)query.set('cursor',cursor);
   const page=await api<Page>(`/jobs?${query}`);if(!live.current)return page.items;
   setJobs(previous=>append?[...previous,...page.items]:page.items);setCursor(page.next_cursor??null);setError('');
   return page.items;
  }catch(value){if(live.current)setError(requestErrorMessage(value));return [] as Job[];}
 }
 function schedule(values:Job[]){
  if(document.visibilityState!=='visible')return;
  timer.current=setTimeout(async()=>{const next=await fetchJobs();schedule(next);},values.some(job=>activeStates.has(job.state))?2000:30000);
 }
 useEffect(()=>{
  live.current=true;
  const refresh=async()=>{if(timer.current)clearTimeout(timer.current);const next=await fetchJobs();schedule(next);};
  const visible=()=>{if(document.visibilityState==='visible')void refresh();else if(timer.current)clearTimeout(timer.current);};
  void refresh();window.addEventListener('online',refresh);document.addEventListener('visibilitychange',visible);
  return()=>{live.current=false;if(timer.current)clearTimeout(timer.current);window.removeEventListener('online',refresh);document.removeEventListener('visibilitychange',visible);};
 },[]);
 async function mutate(job:Job,action:'cancel'|'retry'){
  setChanging(job.id);setError('');
  try{await api(`/jobs/${job.id}/${action}`,{method:'POST'});const next=await fetchJobs();if(timer.current)clearTimeout(timer.current);schedule(next);window.dispatchEvent(new Event('collector-jobs-updated'));}
  catch(value){setError(requestErrorMessage(value));}finally{setChanging('');}
 }
 const active=jobs.filter(job=>activeStates.has(job.state));const history=jobs.filter(job=>!activeStates.has(job.state));
 return <section className="job-ledger" aria-labelledby="job-ledger-heading">
  <div className="section-heading"><div><h3 id="job-ledger-heading">Background work</h3><p className="muted">Your package, publication, and integrity jobs stay here across refreshes.</p></div><button disabled={!!changing} onClick={()=>void fetchJobs()}>Refresh jobs</button></div>
  {error&&<p className="notice error" role="alert">{error}</p>}
  <JobGroup heading="Active" jobs={active} changing={changing} onAction={mutate}/>
  <JobGroup heading="History" jobs={history} changing={changing} onAction={mutate}/>
  {cursor&&<button className="load-more" onClick={()=>void fetchJobs(true)}>Load older jobs</button>}
 </section>;
}

function JobGroup({heading,jobs,changing,onAction}:{heading:string;jobs:Job[];changing:string;onAction:(job:Job,action:'cancel'|'retry')=>Promise<void>}){
 return <div className="job-group"><h4>{heading}</h4>{jobs.length?<div className="job-list">{jobs.map(job=><article key={job.id} aria-label={`${kindLabels[job.kind]||readable(job.kind)} ${readable(job.state)}`} aria-live={activeStates.has(job.state)?'polite':'off'}>
  <header><div><strong>{kindLabels[job.kind]||readable(job.kind)}</strong><span>{readable(job.phase)}</span></div><span className={`job-state ${job.state}`}>{readable(job.state)}</span></header>
  <progress max={Math.max(job.total_units,1)} value={job.total_units?job.completed_units:job.state==='completed'?1:0} aria-label={`${kindLabels[job.kind]||readable(job.kind)} progress`}/><p>{percentage(job)}% · attempt {job.attempt}</p>
  {(job.error||job.limitation)&&<p className={job.error?'error':'muted'}>{job.error||job.limitation}</p>}
  {(job.cancellable||job.retryable)&&<div className="row-actions">{job.cancellable&&<button disabled={changing===job.id} onClick={()=>void onAction(job,'cancel')}>Cancel job</button>}{job.retryable&&<button className="primary" disabled={changing===job.id} onClick={()=>void onAction(job,'retry')}>Retry job</button>}</div>}
 </article>)}</div>:<p className="muted">No {heading.toLowerCase()} jobs.</p>}</div>;
}
