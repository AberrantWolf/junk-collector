import React from '@collector/runtime';
import type {components} from './generated/api';
import {presentationLabel} from './presentation-labels';
type Summary=components['schemas']['IngestSummary'];
const states={running:'Active',queued:'Queued',waiting:'Waiting',cancelling:'Cancelling'};
export default function IngestStatus({summary}:{summary:Summary}){
 return <div className="ingest-work" aria-label="Current ingest activity">
  {summary.outcome!=='active'&&<small>{presentationLabel(summary.outcome)}</small>}
  {summary.work.map(work=><small key={`${work.kind}:${work.state}:${work.phase}:${work.shared}`}>
   <strong className={`ingest-work-${work.state}`}>{states[work.state]}</strong> · {presentationLabel(work.kind,'Processing')}
   {work.phase!=='queued'&&<> · {work.state==='waiting'&&['staging','awaiting_source'].includes(work.phase)?'Waiting for upload':presentationLabel(work.phase,'Processing') }</>}
   {work.shared&&<> · Shared download</>}
   {work.jobs>1&&<> · {work.jobs} tasks</>}
  </small>)}
  {summary.outcome==='active'&&!summary.work.length&&<small>{summary.permissions.can_resume_upload?'Waiting for upload':summary.phase==='awaiting_review'?'Waiting to save originals':'Waiting for next stage'}</small>}
 </div>;
}
