import React,{useState} from '@collector/runtime';
import type {components} from './generated/api';
type Purchase=components['schemas']['DigitalPurchase'];
const empty:Purchase={store:null,source_url:null,date:null,cost:null,order_reference:null};
function digits(currency:string){try{return new Intl.NumberFormat(undefined,{style:'currency',currency}).resolvedOptions().maximumFractionDigits??2;}catch{return 2;}}
export default function PurchaseFields({value,onChange,disabled=false}:{value?:Purchase|null;onChange:(value:Purchase)=>void;disabled?:boolean}){
 const [currency,setCurrency]=useState(value?.cost?.currency||'JPY');
 const [price,setPrice]=useState(value?.cost?String(value.cost.minor_units/10**digits(value.cost.currency)):'');
 const update=(patch:Partial<Purchase>)=>onChange({...empty,...value,...patch});
 const changePrice=(amount:string,code=currency)=>{setPrice(amount);const numeric=Number(amount);update({cost:amount!==''&&Number.isFinite(numeric)?{currency:code,minor_units:Math.round(numeric*10**digits(code))}:null});};
 return <fieldset disabled={disabled}><legend>Purchase details (private)</legend><div className="form-grid"><label>Store<input maxLength={4096} value={value?.store||''} onChange={e=>update({store:e.target.value||null})}/></label><label>Purchase date<input type="date" value={value?.date||''} onChange={e=>update({date:e.target.value||null})}/></label><label>Source URL<input maxLength={4096} value={value?.source_url||''} onChange={e=>update({source_url:e.target.value||null})}/></label><label>Order reference<input maxLength={4096} value={value?.order_reference||''} onChange={e=>update({order_reference:e.target.value||null})}/></label><label>Currency<input pattern="[A-Z]{3}" required={price!==''} maxLength={3} value={currency} onChange={e=>{const code=e.target.value.toUpperCase();setCurrency(code);changePrice(price,code);}}/></label><label>Price<input type="number" min="0" step="any" value={price} onChange={e=>changePrice(e.target.value)}/></label></div></fieldset>;
}
