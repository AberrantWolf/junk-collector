import {spawn,stopProcess,deadline} from './support/runtime.mjs';
const scripts={scale:'fx04',smoke:'quick-smoke',integration:'smoke',browse:'browse','audio-checks':'audio-check-status','accuraterip-snapshots':'accuraterip-snapshots','cd-library':'cd-library',actions:'action-menu',scans:'scans',work:'work-history',fx04:'fx04',fx05:'fx05',fx06:'fx06','fx04-review':'fx04-review','local-playback':'local-playback',lists:'collection-list-chooser',accounts:'account-usability',pair:'pair-device',navigation:'navigation',certificate:'certificate','numeric-ip':'numeric-ip','game-catalog':'game-catalog-upload',purchases:'digital-purchases','intake-review':'intake-review','game-audio':'game-audio',documents:'documents','backup-reminder':'backup-reminder','remote-media':'remote-media',queue:'music-queue',context:'work-context',labels:'presentation-labels'};
const requested=process.argv.slice(2);const selected=requested.length?requested:Object.keys(scripts).filter(name=>!['scale','queue','context','labels'].includes(name));
if(selected.some(name=>!scripts[name]))throw Error('Unknown test suite');
for(const name of [...new Set(selected)]){
 const start=Date.now();const child=spawn(process.execPath,[new URL(`./${scripts[name]}.mjs`,import.meta.url).pathname],{stdio:'inherit',env:{...process.env,...name==='scale'?{COLLECTOR_SCALE_COUNT:'3000'}:{}}});
 const ended=new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',(code,signal)=>code===0?resolve():reject(Error(`${name} failed (${signal||code})`)));});
 try{await deadline(ended,name,name==='scale'?1800000:name==='integration'?600000:180000);}finally{await stopProcess(child);}
 console.log(`${name}: ${((Date.now()-start)/1000).toFixed(2)}s`);
}
