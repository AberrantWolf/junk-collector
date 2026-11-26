import React from '@collector/runtime';
import type {components} from './generated/api';
import {opusBitrates,preparationPreset,type ImportPreparationChoice,type PreparationPreset} from './preparation-presets';
type S=components['schemas'];
export * from './preparation-presets';

export function PreparationFormat({value,onChange,disabled}:{value:PreparationPreset;onChange:(v:PreparationPreset)=>void;disabled?:boolean}){
 return <><label>Preparation format<select aria-label="Preparation format" disabled={disabled} value={value==='flac'?'flac':'opus'} onChange={e=>onChange(e.target.value==='flac'?'flac':'128')}><option value="flac">FLAC</option><option value="opus">Opus</option></select></label>{value!=='flac'&&<label>Opus bitrate<select aria-label="Opus bitrate" disabled={disabled} value={value} onChange={e=>onChange(preparationPreset(e.target.value))}>{opusBitrates.map(rate=><option key={rate} value={rate}>{rate} kbps VBR</option>)}</select></label>}</>;
}

export function ImportPreparation({value,onChange,settings,packages,canFlac,disabled}:{value:ImportPreparationChoice;onChange:(v:ImportPreparationChoice)=>void;settings:S['OpusImportDefault']|null|undefined;packages:S['IngestPlanPackage'][];canFlac:boolean;disabled?:boolean}){
 const count=settings?.scope==='cd_and_digital'?packages.length:settings?.scope==='cd'?packages.filter(pkg=>pkg.anchors.some(anchor=>anchor.selected)).length:0;
 const choice=count?`Opus ${settings!.bitrate_kbps} kbps${count<packages.length?` for ${count} of ${packages.length} packages`:''}`:'Save originals without preparation';
 return <label>Audio preparation after import<select aria-label="Audio preparation after import" disabled={disabled} value={value} onChange={e=>onChange(e.target.value as ImportPreparationChoice)}>
  <option value="default">Collection default · {settings===undefined?'Loading…':choice}</option>
  <option value="none">Save originals without preparation</option>
  <option value="flac" disabled={!canFlac}>Prepare FLAC</option>
  {opusBitrates.map(rate=><option value={rate} key={rate}>Prepare Opus {rate} kbps VBR</option>)}
 </select></label>;
}
