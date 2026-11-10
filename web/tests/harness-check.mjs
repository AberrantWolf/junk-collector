// Lifecycle guarantees for test-owned processes; does not launch a browser.
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {closeHttpFixture,runJourneys,spawn,stopProcess,deadline} from './support/runtime.mjs';
const child=spawn(process.execPath,['-e',"process.on('SIGTERM',()=>{});console.log('ready');setInterval(()=>{},1000)"],{stdio:['ignore','pipe','inherit']});
await deadline(new Promise(resolve=>child.stdout.once('data',resolve)),'child ready');
await stopProcess(child,50);assert.equal(child.signalCode,'SIGKILL');
const owner=spawn(process.execPath,['--input-type=module','-e',`import {spawn} from ${JSON.stringify(new URL('./support/runtime.mjs',import.meta.url).href)};const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)']);console.log(child.pid);setInterval(()=>{},1000);`],{stdio:['ignore','pipe','inherit']});
const pid=Number(await deadline(new Promise(resolve=>owner.stdout.once('data',value=>resolve(String(value).trim()))),'owner ready'));
const ended=new Promise(resolve=>owner.once('exit',resolve));owner.kill('SIGINT');await deadline(ended,'owner shutdown');
assert.equal(owner.exitCode,130);assert.throws(()=>process.kill(pid,0),error=>error.code==='ESRCH');
console.log('Harness lifecycle passed: forced reap and ordinary interruption release only owned processes.');

// Even an uncooperative server and an unmanaged child encoder stay in the
// fixture's process group; forced shutdown must not leave the encoder running.
if(process.platform!=='win32'){
 const source="const {spawn}=require('node:child_process');process.on('SIGTERM',()=>{});const child=spawn(process.execPath,['-e',\"process.on('SIGTERM',()=>{});console.log('ready');setInterval(()=>{},1000)\"],{stdio:['ignore','pipe','inherit']});child.stdout.once('data',()=>console.log(child.pid));setInterval(()=>{},1000)";
 const parent=spawn(process.execPath,['-e',source],{stdio:['ignore','pipe','inherit']});
 const descendant=Number(await deadline(new Promise(resolve=>parent.stdout.once('data',value=>resolve(String(value).trim()))),'encoder ready'));
 await stopProcess(parent,50);
 const until=Date.now()+5000;let alive=true;
 while(alive&&Date.now()<until){try{process.kill(descendant,0);await new Promise(resolve=>setTimeout(resolve,20));}catch(error){if(error.code!=='ESRCH')throw error;alive=false;}}
 assert.equal(alive,false,'forced shutdown left a child encoder alive');
 console.log('Forced process-group shutdown reaped the unmanaged child encoder.');
}


// A held provider response must not leave a journey waiting in finally.
let requested;
const requestArrived=new Promise(resolve=>{requested=resolve;});
const http=createServer(()=>requested());
await new Promise((resolve,reject)=>{http.once('error',reject);http.listen(0,'127.0.0.1',resolve);});
try{
 const response=fetch(`http://127.0.0.1:${http.address().port}/held`).then(()=>false,()=>true);
 await deadline(requestArrived,'held fixture request');
 await closeHttpFixture(http,'held provider fixture');
 assert.equal(await deadline(response,'held fixture connection close'),true);
}finally{await closeHttpFixture(http);}

// The runner still fails the batch, but a failure must not prevent later tests.
const scratch=await mkdtemp(path.join(tmpdir(),'collector-harness-'));
try{
 const marker=path.join(scratch,'completed');
 const failures=await runJourneys([
  {name:'failed fixture',command:process.execPath,args:['-e','process.exit(7)'],timeout:5000},
  {name:'stalled fixture',command:process.execPath,args:['-e','setInterval(()=>{},1000)'],timeout:150},
  {name:'independent fixture',command:process.execPath,args:['-e',"require('node:fs').writeFileSync(process.argv[1],'completed')",marker],timeout:5000},
 ],{report:()=>{}});
 assert.deepEqual(failures.map(({name})=>name),['failed fixture','stalled fixture']);
 assert.equal(await readFile(marker,'utf8'),'completed');
}finally{await rm(scratch,{recursive:true,force:true});}
console.log('Held HTTP fixtures shut down and the batch continues after failures and timeouts.');
