import {fileURLToPath} from 'node:url';
import {runJourneys} from './support/runtime.mjs';
import scripts from './journeys.json' with {type:'json'};
const requested=process.argv.slice(2);const selected=requested.length?requested:Object.keys(scripts).filter(name=>!['scale','queue','context','labels'].includes(name));
if(selected.some(name=>!Object.hasOwn(scripts,name)))throw Error('Unknown test suite');
const failures=await runJourneys([...new Set(selected)].map(name=>({
 name,command:process.execPath,args:[fileURLToPath(new URL(`./${scripts[name]}.mjs`,import.meta.url))],
 options:{stdio:'inherit',env:{...process.env,...name==='scale'?{COLLECTOR_SCALE_COUNT:'3000'}:{}}},
 // Intake combines review, cleanup and paced provider settlement.
 timeout:name==='scale'?1800000:['integration','intake-review'].includes(name)?600000:180000,
})));
if(failures.length){
 console.error(`Failed journeys: ${failures.map(({name})=>name).join(', ')}`);
 process.exitCode=1;
}
