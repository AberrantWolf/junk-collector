import {fileURLToPath} from 'node:url';
import {spawn,stopProcess,deadline} from './support/runtime.mjs';
import scripts from './journeys.json' with {type:'json'};
const requested=process.argv.slice(2);const selected=requested.length?requested:Object.keys(scripts).filter(name=>!['scale','queue','context','labels'].includes(name));
if(selected.some(name=>!scripts[name]))throw Error('Unknown test suite');
for(const name of [...new Set(selected)]){
 const start=Date.now();const child=spawn(process.execPath,[fileURLToPath(new URL(`./${scripts[name]}.mjs`,import.meta.url))],{stdio:'inherit',env:{...process.env,...name==='scale'?{COLLECTOR_SCALE_COUNT:'3000'}:{}}});
 const ended=new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',(code,signal)=>code===0?resolve():reject(Error(`${name} failed (${signal||code})`)));});
 // Intake combines review, cleanup and paced provider settlement (up to four minutes).
 try{await deadline(ended,name,name==='scale'?1800000:['integration','intake-review'].includes(name)?600000:180000);}finally{await stopProcess(child);}
 console.log(`${name}: ${((Date.now()-start)/1000).toFixed(2)}s`);
}
