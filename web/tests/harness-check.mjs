// Lifecycle guarantees for test-owned processes; does not launch a browser.
import assert from 'node:assert/strict';
import {spawn,stopProcess,deadline} from './support/runtime.mjs';
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
