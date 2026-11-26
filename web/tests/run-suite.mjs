import {availableParallelism} from 'node:os';
import {fileURLToPath} from 'node:url';
import {runJourneys} from './support/runtime.mjs';
import listed from './journeys.json' with {type:'json'};
// Each journey names a script, and optionally environment settings that pick
// one scenario inside it, a longer time limit, or `"suite": false` to leave it
// out of a plain `npm test` (because it is long, or repeats part of another).
const journeys=Object.fromEntries(Object.entries(listed).map(([name,value])=>[name,typeof value==='string'?{script:value}:value]));
// These measure timing or play real audio, so they run alone after the others.
const serial=new Set(['opus-preparation','local-playback','scale','fx04','remote-media']);
const requested=process.argv.slice(2);
const selected=requested.length?requested:Object.keys(journeys).filter(name=>journeys[name].suite!==false);
const unknown=selected.filter(name=>!Object.hasOwn(journeys,name));
if(unknown.length)throw Error(`Unknown journey: ${unknown.join(', ')}. Known: ${Object.keys(journeys).join(', ')}`);
const jobs=Number(process.env.COLLECTOR_TEST_JOBS||Math.max(1,Math.floor(availableParallelism()/2)));
if(!Number.isSafeInteger(jobs)||jobs<1)throw Error('COLLECTOR_TEST_JOBS must be a positive whole number');
const started=Date.now();
const failures=await runJourneys([...new Set(selected)].map(name=>{
 const {script,env={},timeout=180000}=journeys[name];
 return {name,serial:serial.has(name),timeout,command:process.execPath,
  args:[fileURLToPath(new URL(`./${script}.mjs`,import.meta.url))],
  options:{stdio:'inherit',env:{...process.env,...env}}};
}),{jobs});
console.log(`${selected.length} journeys in ${((Date.now()-started)/1000).toFixed(1)}s, ${jobs} at a time`);
if(failures.length){
 console.error(`Failed journeys: ${failures.map(({name})=>name).join(', ')}`);
 process.exitCode=1;
}
