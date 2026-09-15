import React,{useState,api,isApiRequestError} from '@collector/runtime';
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
type Specimen=components['schemas']['PresentedSpecimen'];
type Component=components['schemas']['Component'];
type Condition=components['schemas']['Condition'];
type Ownership=components['schemas']['ActiveOwnership'];
type Possession=components['schemas']['Possession'];
type ReturnedPossession=components['schemas']['ReturnedPossession'];
type ComponentKind=components['schemas']['ComponentKind'];
type CarrierKind=components['schemas']['CarrierKind'];

type Props={item:Item;busy:boolean;setBusy:(busy:boolean)=>void;onSaved:(item:Item)=>void;onError:(error:unknown)=>void};
type SpecimenDraft={label:string;condition:Condition;ownership:Ownership;possession:Possession;hasAcquisition:boolean;acquisitionDate:string;cost:string;currency:string;source:string;notes:string};
type ComponentDraft={specimenId:string;id?:string;label:string;kind:ComponentKind;position:string;hasCarrier:boolean;carrierLabel:string;carrierKind:CarrierKind};
type LoanDraft={contact:string;lentAt:string;dueAt:string};
type ReturnDraft={returnedAt:string;possession:ReturnedPossession};
type DisposalDraft={date:string;reason:string;possession:ReturnedPossession};

const conditions:Condition[]=['unknown','sealed','excellent','good','fair','poor','incomplete'];
const ownerships:Ownership[]=['owned','borrowed','unknown'];
const possessions:ReturnedPossession[]=['present','elsewhere','unknown'];
const componentKinds:ComponentKind[]=['cartridge','optical_disc','tape','memory_card','case','booklet','insert','other_data','other_physical'];
const carrierKinds:CarrierKind[]=['cartridge','optical_disc','tape','card','other'];
const today=()=>new Date().toISOString().slice(0,10);
const words=(value:string)=>value.replaceAll('_',' ');

function blankSpecimen():SpecimenDraft{return {label:'',condition:'unknown',ownership:'owned',possession:'present',hasAcquisition:false,acquisitionDate:'',cost:'',currency:'JPY',source:'',notes:''};}
function specimenDraft(specimen:Specimen):SpecimenDraft{return {label:specimen.label,condition:specimen.condition,ownership:specimen.ownership.state==='disposed'?'unknown':specimen.ownership.state,possession:specimen.possession,hasAcquisition:!!specimen.acquisition,acquisitionDate:specimen.acquisition?.date||'',cost:specimen.acquisition?.cost?.minor_units?.toString()||'',currency:specimen.acquisition?.cost?.currency||'JPY',source:specimen.acquisition?.source||'',notes:specimen.private_notes||''};}
function blankComponent(specimenId:string,position:number):ComponentDraft{return {specimenId,label:'',kind:'optical_disc',position:String(position),hasCarrier:true,carrierLabel:'',carrierKind:'optical_disc'};}
function componentDraft(specimenId:string,component:Component):ComponentDraft{return {specimenId,id:component.id,label:component.label,kind:component.kind,position:String(component.position),hasCarrier:!!component.carrier,carrierLabel:component.carrier?.label||'',carrierKind:component.carrier?.kind||'optical_disc'};}

export default function InventoryEditor({item,busy,setBusy,onSaved,onError}:Props){
 const [editingSpecimen,setEditingSpecimen]=useState<string|'new'>();
 const [copyDraft,setCopyDraft]=useState<SpecimenDraft>(blankSpecimen());
 const [partDraft,setPartDraft]=useState<ComponentDraft>();
 const [loanDraft,setLoanDraft]=useState<{specimenId:string;value:LoanDraft}>();
 const [returnDraft,setReturnDraft]=useState<{specimenId:string;loanId:string;value:ReturnDraft}>();
 const [disposalDraft,setDisposalDraft]=useState<{specimenId:string;value:DisposalDraft}>();
 const [conflict,setConflict]=useState<Item>();

 async function mutate(path:string,init:RequestInit){
  setBusy(true);
  try{const saved=await api<Item>(path,init);setConflict(undefined);onSaved(saved);return true;}
  catch(error){
   if(isApiRequestError(error)&&error.status===409){
    try{setConflict(await api<Item>(`/items/${item.id}`));}catch(refreshError){onError(refreshError);}
   }else onError(error);
   return false;
  }finally{setBusy(false);}
 }
 function discardDrafts(){setEditingSpecimen(undefined);setCopyDraft(blankSpecimen());setPartDraft(undefined);setLoanDraft(undefined);setReturnDraft(undefined);setDisposalDraft(undefined);setConflict(undefined);}
 function reloadServer(){if(conflict){onSaved(conflict);discardDrafts();}}
 function reapplyDraft(){if(conflict){onSaved(conflict);setConflict(undefined);}}
 function editCopy(specimen?:Specimen){setEditingSpecimen(specimen?.id||'new');setCopyDraft(specimen?specimenDraft(specimen):blankSpecimen());setPartDraft(undefined);}
 async function saveCopy(event:React.FormEvent){
  event.preventDefault();
  const details={label:copyDraft.label,condition:copyDraft.condition,ownership:copyDraft.ownership,possession:copyDraft.possession,acquisition:copyDraft.hasAcquisition?{date:copyDraft.acquisitionDate||null,cost:copyDraft.cost?{minor_units:Number(copyDraft.cost),currency:copyDraft.currency.toUpperCase()}:null,source:copyDraft.source||null,coordinates:null}:null,private_notes:copyDraft.notes||null};
  const path=editingSpecimen==='new'?`/items/${item.id}/specimens`:`/items/${item.id}/specimens/${editingSpecimen}`;
  if(await mutate(path,{method:editingSpecimen==='new'?'POST':'PUT',body:JSON.stringify({expected_revision:item.revision,details})})){setEditingSpecimen(undefined);setCopyDraft(blankSpecimen());}
 }
 async function savePart(event:React.FormEvent){
  event.preventDefault();if(!partDraft)return;
  const details={label:partDraft.label,kind:partDraft.kind,position:Number(partDraft.position),carrier:partDraft.hasCarrier?{label:partDraft.carrierLabel||partDraft.label,kind:partDraft.carrierKind}:null};
  const base=`/items/${item.id}/specimens/${partDraft.specimenId}/components`;
  if(await mutate(partDraft.id?`${base}/${partDraft.id}`:base,{method:partDraft.id?'PUT':'POST',body:JSON.stringify({expected_revision:item.revision,details})}))setPartDraft(undefined);
 }
 async function removePart(specimenId:string,component:Component){
  if(!window.confirm(`Remove “${component.label}”? Captures and assets must be detached first.`))return;
  await mutate(`/items/${item.id}/specimens/${specimenId}/components/${component.id}?expected_revision=${item.revision}`,{method:'DELETE'});
 }
 async function startLoan(event:React.FormEvent){
  event.preventDefault();if(!loanDraft)return;
  const {specimenId,value}=loanDraft;
  if(await mutate(`/items/${item.id}/specimens/${specimenId}/loans`,{method:'POST',body:JSON.stringify({expected_revision:item.revision,command:{contact:value.contact,lent_at:value.lentAt,due_at:value.dueAt||null}})}))setLoanDraft(undefined);
 }
 async function returnLoan(event:React.FormEvent){
  event.preventDefault();if(!returnDraft)return;
  const {specimenId,loanId,value}=returnDraft;
  if(await mutate(`/items/${item.id}/specimens/${specimenId}/loans/${loanId}/return`,{method:'POST',body:JSON.stringify({expected_revision:item.revision,command:{returned_at:value.returnedAt,possession:value.possession}})}))setReturnDraft(undefined);
 }
 async function dispose(event:React.FormEvent){
  event.preventDefault();if(!disposalDraft||!window.confirm('Record this copy as disposed? Its history and archive files will be retained.'))return;
  const {specimenId,value}=disposalDraft;
  if(await mutate(`/items/${item.id}/specimens/${specimenId}/dispose`,{method:'POST',body:JSON.stringify({expected_revision:item.revision,command:{date:value.date,reason:value.reason,possession:value.possession}})}))setDisposalDraft(undefined);
 }

 return <section data-unsaved={!!editingSpecimen||!!partDraft||!!loanDraft||!!returnDraft||!!disposalDraft} className="inventory-editor" aria-labelledby="physical-inventory-title">
  <div className="section-heading"><div><p className="eyebrow">Copy-level records</p><h3 id="physical-inventory-title">Physical inventory</h3></div><button disabled={busy||!!editingSpecimen} onClick={()=>editCopy()}>Add physical copy</button></div>
  {!item.specimens.length&&!editingSpecimen&&<p className="muted">No physical copies recorded. This item can remain wishlist-only.</p>}
  {conflict&&<div className="notice conflict" role="alert"><strong>Revision conflict</strong><p>Your draft is still here. The server is now at revision {conflict.revision}; this form started from revision {item.revision}.</p><div className="button-row"><button onClick={reloadServer}>Reload and discard draft</button><button className="primary" onClick={reapplyDraft}>Keep draft on revision {conflict.revision}</button></div></div>}
  {editingSpecimen&&<CopyForm draft={copyDraft} setDraft={setCopyDraft} specimen={editingSpecimen==='new'?undefined:item.specimens.find(value=>value.id===editingSpecimen)} busy={busy} onSubmit={saveCopy} onCancel={()=>{setEditingSpecimen(undefined);setCopyDraft(blankSpecimen());}}/>}
  <div className="copy-ledger">{item.specimens.map((specimen,index)=><article className="copy-record" key={specimen.id} aria-labelledby={`copy-${specimen.id}`}>
   <header><div><span className="copy-number">Copy {index+1}</span><h4 id={`copy-${specimen.id}`}>{specimen.label}</h4><p>{words(specimen.condition)} · {specimen.ownership.state==='disposed'?`disposed ${specimen.ownership.date}`:words(specimen.ownership.state)} · {words(specimen.possession)}</p></div>{specimen.ownership.state!=='disposed'&&<button disabled={busy||!!editingSpecimen} onClick={()=>editCopy(specimen)}>Edit copy</button>}</header>
   {specimen.acquisition&&<dl className="copy-facts"><dt>Acquired</dt><dd>{specimen.acquisition.date||'Date unknown'}</dd><dt>Source</dt><dd>{specimen.acquisition.source||'Not recorded'}</dd><dt>Cost</dt><dd>{specimen.acquisition.cost?`${specimen.acquisition.cost.minor_units} ${specimen.acquisition.cost.currency} (minor units)`:'Not recorded'}</dd></dl>}
   {specimen.private_notes&&<p className="private-note"><span>Private note</span>{specimen.private_notes}</p>}
   <div className="record-block"><div className="record-heading"><h5>Parts and carriers</h5>{specimen.ownership.state!=='disposed'&&<button disabled={busy||!!partDraft} onClick={()=>setPartDraft(blankComponent(specimen.id,specimen.components.length+1))}>Add part</button>}</div>
    {!specimen.components.length&&<p className="muted">No parts recorded.</p>}
    <ol className="component-list">{[...specimen.components].sort((a,b)=>a.position-b.position).map(component=><li key={component.id}><span className="part-position">{component.position}</span><div><strong>{component.label}</strong><small>{words(component.kind)}{component.carrier?` · ${component.carrier.label} (${words(component.carrier.kind)})`:''}</small></div>{specimen.ownership.state!=='disposed'&&<div className="row-actions"><button disabled={busy||!!partDraft} onClick={()=>setPartDraft(componentDraft(specimen.id,component))}>Edit</button><button disabled={busy} onClick={()=>void removePart(specimen.id,component)}>Remove</button></div>}</li>)}</ol>
    {partDraft?.specimenId===specimen.id&&<PartForm draft={partDraft} setDraft={setPartDraft} busy={busy} onSubmit={savePart} onCancel={()=>setPartDraft(undefined)}/>} 
   </div>
   <div className="record-block"><div className="record-heading"><h5>Loan history</h5>{specimen.ownership.state!=='disposed'&&!specimen.loans?.some(loan=>!loan.returned_at)&&<button disabled={busy||!!loanDraft} onClick={()=>setLoanDraft({specimenId:specimen.id,value:{contact:'',lentAt:today(),dueAt:''}})}>Lend copy</button>}</div>
    {!specimen.loans?.length&&<p className="muted">No loans recorded.</p>}
    <ol className="loan-history">{specimen.loans?.map(loan=><li key={loan.id}><div><strong>{loan.contact}</strong><small>Lent {loan.lent_at}{loan.due_at?` · due ${loan.due_at}`:''}{loan.returned_at?` · returned ${loan.returned_at}`:' · currently lent'}</small></div>{!loan.returned_at&&<button disabled={busy||!!returnDraft} onClick={()=>setReturnDraft({specimenId:specimen.id,loanId:loan.id,value:{returnedAt:today(),possession:'present'}})}>Return copy</button>}</li>)}</ol>
    {loanDraft?.specimenId===specimen.id&&<LoanForm draft={loanDraft.value} setDraft={value=>setLoanDraft({...loanDraft,value})} busy={busy} onSubmit={startLoan} onCancel={()=>setLoanDraft(undefined)}/>} 
    {returnDraft?.specimenId===specimen.id&&<ReturnForm draft={returnDraft.value} setDraft={value=>setReturnDraft({...returnDraft,value})} busy={busy} onSubmit={returnLoan} onCancel={()=>setReturnDraft(undefined)}/>} 
   </div>
   {specimen.ownership.state!=='disposed'&&<div className="record-block danger-zone"><div className="record-heading"><h5>Lifecycle endpoint</h5><button disabled={busy||!!disposalDraft||specimen.loans?.some(loan=>!loan.returned_at)} onClick={()=>setDisposalDraft({specimenId:specimen.id,value:{date:today(),reason:'',possession:'elsewhere'}})}>Record disposal</button></div>{specimen.loans?.some(loan=>!loan.returned_at)&&<p className="muted">Return the active loan before recording disposal.</p>}{disposalDraft?.specimenId===specimen.id&&<DisposalForm draft={disposalDraft.value} setDraft={value=>setDisposalDraft({...disposalDraft,value})} busy={busy} onSubmit={dispose} onCancel={()=>setDisposalDraft(undefined)}/>}</div>}
  </article>)}</div>
 </section>;
}

function CopyForm({draft,setDraft,specimen,busy,onSubmit,onCancel}:{draft:SpecimenDraft;setDraft:(value:SpecimenDraft)=>void;specimen?:Specimen;busy:boolean;onSubmit:(event:React.FormEvent)=>void;onCancel:()=>void}){
 const activeLoan=specimen?.loans?.some(loan=>!loan.returned_at);
 return <form aria-label={specimen?'Edit physical copy':'New physical copy'} className="ledger-form" onSubmit={onSubmit}><h4>{specimen?'Edit copy':'New physical copy'}</h4><div className="form-grid"><label>Copy label<input required disabled={busy} value={draft.label} onChange={event=>setDraft({...draft,label:event.target.value})}/></label><label>Condition<select disabled={busy} value={draft.condition} onChange={event=>setDraft({...draft,condition:event.target.value as Condition})}>{conditions.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label><label>Ownership<select disabled={busy} value={draft.ownership} onChange={event=>setDraft({...draft,ownership:event.target.value as Ownership})}>{ownerships.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label><label>Possession<select disabled={busy||activeLoan} value={draft.possession} onChange={event=>setDraft({...draft,possession:event.target.value as Possession})}>{activeLoan&&<option value="lent">lent</option>}{possessions.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label></div><label className="inline-check"><input type="checkbox" disabled={busy} checked={draft.hasAcquisition} onChange={event=>setDraft({...draft,hasAcquisition:event.target.checked})}/> Record acquisition details</label>{draft.hasAcquisition&&<div className="form-grid acquisition-fields"><label>Acquisition date<input type="date" disabled={busy} value={draft.acquisitionDate} onChange={event=>setDraft({...draft,acquisitionDate:event.target.value})}/></label><label>Source<input disabled={busy} value={draft.source} onChange={event=>setDraft({...draft,source:event.target.value})}/></label><label>Cost in minor units<input type="number" step="1" disabled={busy} value={draft.cost} onChange={event=>setDraft({...draft,cost:event.target.value})}/></label><label>Currency<input maxLength={3} disabled={busy} value={draft.currency} onChange={event=>setDraft({...draft,currency:event.target.value})}/></label></div>}<label>Private notes<textarea disabled={busy} value={draft.notes} onChange={event=>setDraft({...draft,notes:event.target.value})}/></label><div className="button-row"><button type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="primary" disabled={busy}>Save copy</button></div></form>;
}

function PartForm({draft,setDraft,busy,onSubmit,onCancel}:{draft:ComponentDraft;setDraft:(value:ComponentDraft)=>void;busy:boolean;onSubmit:(event:React.FormEvent)=>void;onCancel:()=>void}){
 const dataBearing=['cartridge','optical_disc','tape','memory_card','other_data'].includes(draft.kind);
 return <form aria-label={draft.id?'Edit part':'New part'} className="ledger-form inset" onSubmit={onSubmit}><h5>{draft.id?'Edit part':'New part'}</h5><div className="form-grid"><label>Part label<input required disabled={busy} value={draft.label} onChange={event=>setDraft({...draft,label:event.target.value})}/></label><label>Order<input required min="0" max="65535" type="number" disabled={busy} value={draft.position} onChange={event=>setDraft({...draft,position:event.target.value})}/></label><label>Part kind<select disabled={busy} value={draft.kind} onChange={event=>{const kind=event.target.value as ComponentKind;setDraft({...draft,kind,hasCarrier:draft.hasCarrier&&['cartridge','optical_disc','tape','memory_card','other_data'].includes(kind)});}}>{componentKinds.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label></div>{dataBearing&&<><label className="inline-check"><input type="checkbox" disabled={busy} checked={draft.hasCarrier} onChange={event=>setDraft({...draft,hasCarrier:event.target.checked})}/> Record data-bearing carrier</label>{draft.hasCarrier&&<div className="form-grid"><label>Carrier label<input required disabled={busy} value={draft.carrierLabel} onChange={event=>setDraft({...draft,carrierLabel:event.target.value})}/></label><label>Carrier kind<select disabled={busy} value={draft.carrierKind} onChange={event=>setDraft({...draft,carrierKind:event.target.value as CarrierKind})}>{carrierKinds.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label></div>}</>}<div className="button-row"><button type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="primary" disabled={busy}>Save part</button></div></form>;
}

function LoanForm({draft,setDraft,busy,onSubmit,onCancel}:{draft:LoanDraft;setDraft:(value:LoanDraft)=>void;busy:boolean;onSubmit:(event:React.FormEvent)=>void;onCancel:()=>void}){return <form aria-label="Start loan" className="ledger-form inset" onSubmit={onSubmit}><h5>Start loan</h5><label>Contact<input required disabled={busy} value={draft.contact} onChange={event=>setDraft({...draft,contact:event.target.value})}/></label><div className="form-grid"><label>Lent date<input required type="date" disabled={busy} value={draft.lentAt} onChange={event=>setDraft({...draft,lentAt:event.target.value})}/></label><label>Due date<input type="date" disabled={busy} value={draft.dueAt} onChange={event=>setDraft({...draft,dueAt:event.target.value})}/></label></div><div className="button-row"><button type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="primary" disabled={busy}>Lend copy</button></div></form>;}
function ReturnForm({draft,setDraft,busy,onSubmit,onCancel}:{draft:ReturnDraft;setDraft:(value:ReturnDraft)=>void;busy:boolean;onSubmit:(event:React.FormEvent)=>void;onCancel:()=>void}){return <form aria-label="Return loan" className="ledger-form inset" onSubmit={onSubmit}><h5>Return copy</h5><div className="form-grid"><label>Return date<input required type="date" disabled={busy} value={draft.returnedAt} onChange={event=>setDraft({...draft,returnedAt:event.target.value})}/></label><label>Resulting possession<select disabled={busy} value={draft.possession} onChange={event=>setDraft({...draft,possession:event.target.value as ReturnedPossession})}>{possessions.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label></div><div className="button-row"><button type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="primary" disabled={busy}>Return copy</button></div></form>;}
function DisposalForm({draft,setDraft,busy,onSubmit,onCancel}:{draft:DisposalDraft;setDraft:(value:DisposalDraft)=>void;busy:boolean;onSubmit:(event:React.FormEvent)=>void;onCancel:()=>void}){return <form aria-label="Record disposal" className="ledger-form inset disposal-form" onSubmit={onSubmit}><h5>Record disposal without deleting history</h5><div className="form-grid"><label>Disposal date<input required type="date" disabled={busy} value={draft.date} onChange={event=>setDraft({...draft,date:event.target.value})}/></label><label>Resulting possession<select disabled={busy} value={draft.possession} onChange={event=>setDraft({...draft,possession:event.target.value as ReturnedPossession})}>{possessions.map(value=><option value={value} key={value}>{words(value)}</option>)}</select></label></div><label>Reason<textarea required disabled={busy} value={draft.reason} onChange={event=>setDraft({...draft,reason:event.target.value})}/></label><div className="button-row"><button type="button" disabled={busy} onClick={onCancel}>Cancel</button><button className="primary" disabled={busy}>Record disposal</button></div></form>;}
