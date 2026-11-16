import React from '@collector/runtime';
import type {components} from './generated/api';

type S=components['schemas'];
export type PreparationPreset='flac'|'96'|'128'|'192';
export type ImportPreparationChoice=PreparationPreset|'default'|'none';
export const opusBitrates=[96,128,192] as const;
export function preparationPreset(value:string|null):PreparationPreset{
 return value==='96'||value==='128'||value==='192'?value:'flac';
}
export function conversionProfile(value:PreparationPreset):S['ConversionProfile']{
 return value==='flac'?{codec:'flac'}:{codec:'opus',bitrate_kbps:Number(value)};
}
export function profileLabel(profile:S['ConversionProfile']|null|undefined){
 return !profile?'Original':profile.codec==='flac'?'FLAC':`Opus ${profile.bitrate_kbps} kbps`;
}
export function preparationQuery(preset:PreparationPreset){
 return preset==='flac'?'':`opus_bitrate_kbps=${preset}`;
}
export function importPreparation(choice:ImportPreparationChoice):S['ImportAudioPreparation']|undefined{
 if(choice==='default')return undefined;
 return choice==='none'?{codec:'none'}:conversionProfile(choice);
}

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
