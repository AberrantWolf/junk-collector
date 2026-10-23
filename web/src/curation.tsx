import React,{useEffect,useRef,useState,api,isApiRequestError,requestErrorMessage} from '@collector/runtime';
import type {components} from './generated/api';
import InventoryEditor from './inventory-editor';
import AssetEditor from './asset-editor';
import ImportWorkspace from './import-workspace';
import ProcessingWorkspace from './processing-workspace';
import MusicTracks from './music-tracks';
import GuidedIngest from './guided-ingest';
import CleanReimport from './clean-reimport';
import {useDialogDismiss} from './dialog';
import {CollectionChooser,useCollectionSearch} from './collection-chooser';

type Collection=components['schemas']['PresentedCollection'];
type CollectionType=components['schemas']['ModuleDescriptor'];
type DomainField=components['schemas']['DomainFieldDescriptor'];
type Item=components['schemas']['PresentedItem'];
type SaveItemEditCommand=components['schemas']['SaveItemEditCommand'];

type AnalysisTarget={sourceId?:string;operation:'structure'|'accuraterip'};
type Props={initialAnalysis?:AnalysisTarget;initialCollectionId?:string;initialSection?:'details'|'files'|'evidence'|'reimport'|'documents';collections:Collection[];collectionTypes:CollectionType[];selected?:Item;onClose:()=>void;onSaved:(item:Item,replacedItemId?:string)=>void;onSessionExpired:()=>void};
type Feedback={kind:'success'|'error';text:string};

function scalarValues(item?:Item):Record<string,string>{const payload=item?.domain.payload;if(!payload||typeof payload!=='object'||Array.isArray(payload))return {};return Object.fromEntries(Object.entries(payload).filter(([,value])=>typeof value==='string'||typeof value==='number').map(([key,value])=>[key,String(value)]));}
function inventoryTitle(item?:Item):string{return item?.inventory_title??item?.title??'';}
function inventorySubtitle(item?:Item):string{return item?.inventory_subtitle??item?.subtitle??'';}
function editableItem(item:Item):Item{return {...item,title:inventoryTitle(item),subtitle:inventorySubtitle(item)};}

export default function Curation({initialAnalysis,initialCollectionId,initialSection='details',collections,collectionTypes,selected,onClose,onSaved,onSessionExpired}:Props){
 const dialog=useDialogDismiss(onClose);const [addingMode,setAddingMode]=useState<'import'|'manual'>('import');
 const [section,setSection]=useState<'details'|'copies'|'files'|'music'|'artwork'|'documents'|'evidence'|'reimport'>(initialAnalysis?'evidence':initialSection);const [guidedDraft,setGuidedDraft]=useState(false);
 const [technicalOpen,setTechnicalOpen]=useState(!!initialAnalysis);const [analysisTarget,setAnalysisTarget]=useState<AnalysisTarget|undefined>(initialAnalysis);
 const [item,setItem]=useState(selected&&editableItem(selected));const [title,setTitle]=useState(inventoryTitle(selected));const [subtitle,setSubtitle]=useState(inventorySubtitle(selected));const [collection,setCollection]=useState(selected?.collection_id||initialCollectionId||collections[0]?.id||'');const [fieldValues,setFieldValues]=useState<Record<string,string>>(()=>scalarValues(selected));const [domainPatch,setDomainPatch]=useState<Record<string,unknown>>({});const [wishlist,setWishlist]=useState(selected?.wishlist||false);const [feedback,setFeedback]=useState<Feedback>();const [busy,setBusy]=useState(false);
 const adoptedItemId=useRef(selected?.id);
 const activeCollection=item?.collection_id||collection;
 const collectionChoices=useCollectionSearch(activeCollection,{minRole:1},collections.find(value=>value.id===activeCollection),onSessionExpired);
 useEffect(()=>{if(!item&&!collection){const first=collectionChoices.items.find(value=>value.content_permissions.can_edit);if(first)changeCollection(first.id);}},[item,collection,collectionChoices.items]);
 const selectedCollection=collectionChoices.selected;const owner=selectedCollection?.binding.state==='assigned'?selectedCollection.binding.module:'';const descriptor=collectionTypes.find(value=>value.id===owner);const canEdit=!!selectedCollection?.content_permissions.can_edit&&(!item||item.content_permissions.can_edit);

 function fail(error:unknown){
  if(isApiRequestError(error)&&error.code==='unauthenticated'){onSessionExpired();return;}
  setFeedback({kind:'error',text:requestErrorMessage(error,'You are signed in, but your collection role does not allow this change. Ask a collection owner for Curator access.')});
 }
 async function save(){
  setBusy(true);setFeedback(undefined);try{
   if(!canEdit)throw new Error('Content editing is unavailable for this collection or item.');
   for(const field of descriptor?.fields||[]){if(field.kind.type==='integer'&&domainPatch[field.key]!==undefined&&domainPatch[field.key]!==null&&!Number.isSafeInteger(domainPatch[field.key]))throw new Error(`${field.label} must be a safely representable whole number.`);}
   if(!descriptor)throw new Error('The selected collection type is unavailable. Saved content remains read-only.');
   const edit:SaveItemEditCommand={collection_id:item?.collection_id||collection,title,subtitle,identity_link:item?.identity_link||{kind:'none'},domain_edit:{module:descriptor.id,fields:domainPatch as never},identity:item?.identity||'unidentified',translations:item?.translations||[],identifiers:item?.identifiers||[],wishlist};
   const saved=await api<Item>(`/items/${item?.id||crypto.randomUUID()}`,{method:'PUT',body:JSON.stringify({edit,expected_revision:item?.revision||0})});adoptedItemId.current=saved.id;setItem(editableItem(saved));setFieldValues(scalarValues(saved));setDomainPatch({});if(!item&&guidedDraft)setSection('files');onSaved(saved);setFeedback({kind:'success',text:'Entry saved. You can now attach artwork or original files.'});
  }catch(error){fail(error);}finally{setBusy(false);}
 }
 function inventorySaved(saved:Item){setItem(editableItem(saved));onSaved(saved);setFeedback({kind:'success',text:'Physical inventory updated.'});}
 function capturePublished(saved:Item){setItem(editableItem(saved));onSaved(saved);setFeedback({kind:'success',text:saved.domain.module==='org.junk.music'?'Original files saved. Open Music for available tracks and preparation; automatic disc lookup appears in Work & history when a supported CD identity is available.':'Originals saved. Package operations are available in Advanced evidence.'});}
 function assetsSaved(saved:Item){setItem(editableItem(saved));onSaved(saved);setFeedback({kind:'success',text:section==='documents'?'Documents updated.':'Artwork and photos updated.'});}
 function metadataChanged(saved:Item){setItem(editableItem(saved));onSaved(saved);}
 function guidedPublished(published:Item,replacedItemId?:string){
  const firstAdoption=adoptedItemId.current!==published.id;
  adoptedItemId.current=published.id;
  setItem(editableItem(published));
  if(firstAdoption){
   setTitle(inventoryTitle(published));setSubtitle(inventorySubtitle(published));setWishlist(published.wishlist);
   setCollection(published.collection_id);setFieldValues(scalarValues(published));setDomainPatch({});setGuidedDraft(false);
   setSection('files');setFeedback({kind:'success',text:published.domain.module==='org.junk.music'?'Original files saved. Metadata lookup and audio preparation continue in Work & history.':'Original files saved. Identification and metadata lookup continue in Work & history.'});
  }
  onSaved(published,replacedItemId);
 }
 function changeCollection(id:string){if(id===collection)return;if((Object.keys(domainPatch).length>0||guidedDraft)&&!window.confirm('Changing collection clears the current import preview and type-specific fields. Continue?'))return;setCollection(id);setFieldValues({});setDomainPatch({});setGuidedDraft(false);setFeedback({kind:'success',text:'Collection changed. Review the fields and package readiness for the new collection.'});}
 function changeField(field:DomainField,value:string){setFieldValues(current=>({...current,[field.key]:value}));setDomainPatch(current=>({...current,[field.key]:value===''?null:field.kind.type==='integer'?Number(value):value}));}
 return <div className="modal-backdrop"><section ref={dialog.ref} data-unsaved={title!==(item?.title||'')||subtitle!==(item?.subtitle||'')||wishlist!==(item?.wishlist||false)||Object.keys(domainPatch).some(key=>fieldValues[key]!== (scalarValues(item)[key]||''))||guidedDraft} className="modal curation-modal" role="dialog" aria-modal="true" aria-labelledby="curation-title" aria-busy={busy}><button className="close" onClick={dialog.dismiss} aria-label="Close curator">×</button><h2 id="curation-title">{item?'Edit entry':'Add or import'}</h2>{(item?.content_permissions.message||selectedCollection?.content_permissions.message)&&<p className="notice" role="note">{item?.content_permissions.message||selectedCollection?.content_permissions.message}</p>}{feedback&&<p role={feedback.kind==='error'?'alert':'status'} className={`notice ${feedback.kind==='error'?'error':''}`}>{feedback.text}</p>}{item&&<nav className="curation-sections" aria-label="Editing tasks">{(['details','copies','files','music','artwork','documents','evidence','reimport'] as const).filter(value=>value!=='music'||item.supports_listening).map(value=><button key={value} type="button" aria-current={section===value?'page':undefined} onClick={()=>setSection(value)}>{value==='reimport'?'Replace files':value==='evidence'?'Advanced evidence':value[0].toUpperCase()+value.slice(1)}</button>)}</nav>}{!item&&<div className="adding-mode"><button disabled={busy} aria-pressed={addingMode==='import'} onClick={()=>setAddingMode('import')}>Import original files</button><button disabled={busy} aria-pressed={addingMode==='manual'} onClick={()=>setAddingMode('manual')}>Add entry manually</button></div>}{!item&&addingMode==='import'&&<CollectionChooser label="Collection" value={collection} onChange={changeCollection} choices={collectionChoices} selectable={value=>value.content_permissions.can_edit} disabled={busy}/>}{activeCollection&&canEdit&&<div hidden={item?section!=='files':addingMode!=='import'}><GuidedIngest key={`ingests:${activeCollection}`} initialItem={item} collectionId={activeCollection} busy={busy} setBusy={setBusy} onDraftChange={setGuidedDraft} onPublished={guidedPublished} onError={fail}/></div>} {(item||addingMode==='manual')&&<><div hidden={!!item&&section!=='details'}><h3>{item?'Entry details':'Add entry without files'}</h3><form onSubmit={event=>{event.preventDefault();void save();}}>{item?<p>Collection: {selectedCollection?.name||"Loading collection…"}</p>:<CollectionChooser label="Collection" value={collection} onChange={changeCollection} choices={collectionChoices} selectable={value=>value.content_permissions.can_edit} disabled={busy}/>}<label>Title<input required disabled={busy||!descriptor||!canEdit} value={title} onChange={event=>setTitle(event.target.value)}/></label><label>Edition / variant details<input disabled={busy||!descriptor||!canEdit} value={subtitle} onChange={event=>setSubtitle(event.target.value)}/></label>{descriptor?.fields.map(field=><DomainFieldInput key={field.key} field={field} value={fieldValues[field.key]||''} busy={busy||!canEdit} onChange={value=>changeField(field,value)}/>)}<div className="checks"><label><input type="checkbox" disabled={busy||!descriptor||!canEdit} checked={wishlist} onChange={event=>setWishlist(event.target.checked)}/> Wishlist</label></div><button className="primary" disabled={busy||!collection||!descriptor||!canEdit}>{item?'Save details':'Add entry'}</button></form></div></>}{item&&item.content_permissions.can_edit&&<><div hidden={section!=='reimport'}><CleanReimport key={item.id} item={item} busy={busy} setBusy={setBusy} onPublished={metadataChanged} onError={fail}/></div><div hidden={section!=='copies'}><InventoryEditor item={item} busy={busy} setBusy={setBusy} onSaved={inventorySaved} onError={fail}/></div><div hidden={section!=='artwork'}><AssetEditor key={`${item.id}/images`} item={item} busy={busy} setBusy={setBusy} onSaved={assetsSaved} onError={fail}/></div><div hidden={section!=='documents'}><AssetEditor key={`${item.id}/documents`} mode="documents" item={item} busy={busy} setBusy={setBusy} onSaved={assetsSaved} onError={fail} onSessionExpired={onSessionExpired}/></div><div hidden={section!=='music'}>{item.supports_listening&&<><MusicTracks key={item.id} item={item} canPrepare onPrepared={metadataChanged} onOpenFiles={()=>setSection('files')} onOpenMetadata={()=>{if(dialog.dismiss())window.dispatchEvent(new CustomEvent('collector-open-metadata',{detail:{item:item.id,collection:item.collection_id}}));}} onOpenAnalysis={(sourceId,operation)=>{setAnalysisTarget({sourceId,operation});setTechnicalOpen(true);setSection('evidence');}} onOpenHistory={()=>{if(dialog.dismiss())window.dispatchEvent(new Event('collector-open-work'));}} onSessionExpired={onSessionExpired}/><button onClick={()=>{if(dialog.dismiss())window.dispatchEvent(new CustomEvent('collector-open-metadata',{detail:{item:item.id,collection:item.collection_id}}));}}>Open Metadata workspace</button></>}</div><div hidden={section!=='evidence'}><button onClick={()=>{if(dialog.dismiss())window.dispatchEvent(new CustomEvent('collector-open-metadata',{detail:{item:item.id,collection:item.collection_id}}));}}>Review metadata</button><details open={technicalOpen} onToggle={event=>setTechnicalOpen(event.currentTarget.open)}><summary>Technical processing history</summary>{technicalOpen&&<ProcessingWorkspace item={item} busy={busy} setBusy={setBusy} onError={fail} focusSourceId={analysisTarget?.sourceId} focusOperation={analysisTarget?.operation}/>}</details><details><summary>Advanced manual import review</summary><ImportWorkspace key={item.id} item={item} onPublished={capturePublished} onError={fail}/></details><button onClick={()=>{if(dialog.dismiss())window.dispatchEvent(new Event('collector-open-work'));}}>Open work &amp; history for this entry</button></div></>}</section></div>;
}

function DomainFieldInput({field,value,busy,onChange}:{field:DomainField;value:string;busy:boolean;onChange:(value:string)=>void}){
 if(field.kind.type==='choice')return <label>{field.label}<select aria-label={field.label} disabled={busy} required={field.required} value={value} onChange={event=>onChange(event.target.value)}><option value="">Automatic / unspecified</option>{value&&!field.kind.values.includes(value)&&<option value={value}>{value} (historical)</option>}{field.kind.values.map(choice=><option key={choice} value={choice}>{choice}</option>)}</select></label>;
 if(field.kind.type==='structured')return <div className="read-only-field"><strong>{field.label}</strong><span>This information is saved but cannot be edited here.</span></div>;
 if(!field.editable)return <div className="read-only-field"><strong>{field.label}</strong><span>This field cannot be edited here.</span></div>;
 return <label>{field.label}<input disabled={busy} required={field.required} type={field.kind.type==='integer'?'number':'text'} min={field.kind.type==='integer'?field.kind.minimum:undefined} max={field.kind.type==='integer'?field.kind.maximum:undefined} maxLength={field.kind.type==='text'?field.kind.maximum_bytes:undefined} value={value} onChange={event=>onChange(event.target.value)}/></label>;
}
