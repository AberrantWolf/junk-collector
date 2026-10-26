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
