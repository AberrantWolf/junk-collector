// Explicit launch/lifecycle qualification, outside the macOS seatbelt sandbox.
import {chromium} from './support/browser.mjs';
import {spawn,deadline,stopProcess} from './support/runtime.mjs';
import assert from 'node:assert/strict';
for(let index=0;index<10;index++){
 const browser=await chromium.launch();const child=browser.fixtureProcess;
 try{const page=await browser.newPage();await page.goto('data:text/html,<button>Ready</button>');await page.getByRole('button',{name:'Ready'}).click();}finally{await browser.close();}
 assert.ok(child.exitCode!==null||child.signalCode!==null,'browser was not reaped');
}
for(const interrupt of [false,true]){
 const source=`import {chromium} from ${JSON.stringify(new URL('./support/browser.mjs',import.meta.url).href)};const browser=await chromium.launch();console.log(browser.fixtureProcess.pid);${interrupt?'setInterval(()=>{},1000)':'throw Error("Intentional lifecycle assertion failure")'};`;
 const owner=spawn(process.execPath,['--input-type=module','-e',source],{stdio:['ignore','pipe','pipe']});let logs='';owner.stderr.on('data',value=>logs+=value);
 const ended=new Promise(resolve=>owner.once('exit',resolve));
 try{
  const pid=Number(await deadline(new Promise(resolve=>owner.stdout.once('data',value=>resolve(String(value).trim()))),'browser owner ready'));
  if(interrupt)owner.kill('SIGINT');await deadline(ended,'browser owner cleanup');
  assert.equal(owner.exitCode,interrupt?130:1,logs);assert.throws(()=>process.kill(pid,0),error=>error.code==='ESRCH','owned browser survived failure/interruption');
 }finally{await stopProcess(owner);}
}
console.log('Browser lifecycle passed: ten launches/exits, assertion failure and interruption; all owned browser processes reaped.');
