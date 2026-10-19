import React,{useState,api,requestErrorMessage,useOperationScope} from '@collector/runtime';
import type {components} from './generated/api';
import PurchaseFields from './purchase-fields';
type Capture=components['schemas']['PresentedCapture'];
type Purchase=components['schemas']['DigitalPurchase'];
export default function PurchaseDetails({itemId,capture,onChanged}:{itemId:string;capture:Capture;onChanged:()=>void}){
 const scope=useOperationScope(`${itemId}/${capture.id}/${capture.revision}`);
 const [editing,setEditing]=useState(false),[draft,setDraft]=useState<Purchase|null>(capture.purchase||null),[busy,setBusy]=useState(false),[error,setError]=useState('');
 async function save(purchase:Purchase|null){setBusy(true);setError('');await scope.run(attempt=>api(`/items/${itemId}/captures/${capture.id}/purchase`,{method:'PUT',signal:attempt.signal,body:JSON.stringify({expected_revision:capture.revision,purchase})}),{success:()=>{setEditing(false);onChanged();},failure:e=>setError(requestErrorMessage(e)),settled:()=>setBusy(false)});}

 return <details><summary>Purchase details (private)</summary>{error&&<p role="alert">{error}</p>}{editing?<form onSubmit={e=>{e.preventDefault();void save(draft);}}><PurchaseFields value={draft} disabled={busy} onChange={setDraft}/><button disabled={busy}>Save purchase details</button><button type="button" disabled={busy} onClick={()=>void save(null)}>Clear purchase details</button><button type="button" disabled={busy} onClick={()=>setEditing(false)}>Cancel purchase edit</button></form>:<><dl>{capture.purchase?.store&&<><dt>Store</dt><dd>{capture.purchase.store}</dd></>}{capture.purchase?.date&&<><dt>Date</dt><dd>{capture.purchase.date}</dd></>}{capture.purchase?.source_url&&<><dt>Source</dt><dd>{capture.purchase.source_url}</dd></>}{capture.purchase?.order_reference&&<><dt>Order reference</dt><dd>{capture.purchase.order_reference}</dd></>}{capture.purchase?.cost&&<><dt>Price</dt><dd>{new Intl.NumberFormat(undefined,{style:'currency',currency:capture.purchase.cost.currency}).format(capture.purchase.cost.minor_units/10**(new Intl.NumberFormat(undefined,{style:'currency',currency:capture.purchase.cost.currency}).resolvedOptions().maximumFractionDigits??2))}</dd></>}</dl><button onClick={()=>{setDraft(capture.purchase||null);setEditing(true);}}>Edit purchase details</button></>}</details>;
}
