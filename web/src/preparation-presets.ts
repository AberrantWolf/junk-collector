// Audio preparation choices and the request values they turn into. No UI here,
// so URL parsing and unit tests can use them without React.
import type {components} from './generated/api';

type S=components['schemas'];
export const opusBitrates=[64,96,128,160,192,256] as const;
export type PreparationPreset='flac'|`${typeof opusBitrates[number]}`;
export type ImportPreparationChoice=PreparationPreset|'default'|'none';
export function preparationPreset(value:string|null):PreparationPreset{
 return opusBitrates.some(rate=>String(rate)===value)?value as PreparationPreset:'flac';
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
