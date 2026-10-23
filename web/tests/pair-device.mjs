import {chromium,expect} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-pair-'));
const config=path.join(scratch,'collector.toml');const password=randomBytes(24).toString('hex');
const multiUser=process.env.COLLECTOR_MULTI_USER==='1';const base='http://localhost:19942';const binary=path.join(root,'target/debug/collector-server');
await writeFile(config,`multi_user = ${multiUser}\narchive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19941"\nprivate_bind = "127.0.0.1:19942"\nlan_bind = "127.0.0.1:19943"\nlocal_playback_bind = "127.0.0.1:0"\nallowed_origins = ["${base}"]\n`);
const setup=spawnSync(binary,['--config',config,'setup'],{input:password+'\n',encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let browser;let server;let logs='';
try{
 server=spawn(binary,['--config',config,'serve']);server.stdout.on('data',value=>logs+=value);server.stderr.on('data',value=>logs+=value);
 let ready=false;for(let i=0;i<100;i++){try{if((await fetch(base+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const context=await browser.newContext();const page=await context.newPage();const errors=[];page.on('pageerror',value=>errors.push(value.message));
 const started=await context.request.post(base+'/api/v1/enrollments/start',{data:{device_name:'Trash Amp pairing test'}});assert.equal(started.status(),200);const enrollment=await started.json();
 await page.goto(base+enrollment.verification_url+'?code='+encodeURIComponent(enrollment.user_code));if(multiUser)await page.getByLabel('Username',{exact:true}).fill('owner');await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('heading',{name:'Connect Trash Amp',exact:true}).waitFor();await expect(page.getByLabel('Approval code',{exact:true})).toHaveValue(enrollment.user_code);await expect(page.getByText('Signed in as',{exact:false})).toContainText('owner');
 await page.getByLabel('Approval code',{exact:true}).fill('WRONG-CODE');await page.getByRole('button',{name:'Review connection',exact:true}).click();await page.getByRole('alert').filter({hasText:'That approval code is unavailable for this account'}).waitFor();
 if(multiUser){const created=await context.request.post(base+'/api/v1/admin/users',{headers:{origin:base,'idempotency-key':randomBytes(16).toString('hex')},data:{username:'second',password,administrator:false}});assert.equal(created.status(),200);}
 await page.getByRole('button',{name:'Sign in as another account',exact:true}).click();if(multiUser)await page.getByLabel('Username',{exact:true}).fill('second');await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('heading',{name:'Connect Trash Amp',exact:true}).waitFor();
 await page.getByLabel('Approval code',{exact:true}).fill(enrollment.user_code);await page.getByRole('button',{name:'Review connection',exact:true}).click();await expect(page.getByText('Trash Amp pairing test',{exact:true})).toBeVisible();await page.getByRole('button',{name:'Approve this app',exact:true}).click();await page.getByRole('heading',{name:'App approved',exact:true}).waitFor();
 await page.screenshot({path:'/tmp/collector-pair-device.png',fullPage:true});
 const completed=await context.request.post(`${base}/api/v1/enrollments/${enrollment.id}/complete`,{data:{completion_secret:enrollment.completion_secret}});assert.equal(completed.status(),200);const credential=await completed.json();assert.equal(credential.username,multiUser?'second':'owner');
 const native=await fetch(base+'/api/v1/me',{headers:{authorization:'Bearer '+credential.credential}});assert.equal(native.status,200);
 const replay=await context.request.post(`${base}/api/v1/enrollments/${enrollment.id}/complete`,{data:{completion_secret:enrollment.completion_secret}});assert.notEqual(replay.status(),200);
 assert.deepEqual(errors,[]);console.log('Pairing browser journey passed: signed-out destination, account display, invalid code retry, sign-out destination, approval, native credential and one-time completion.');
}finally{
 await browser?.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(resolve=>server.once('exit',resolve)),new Promise(resolve=>setTimeout(()=>{server.kill('SIGKILL');resolve();},5000))]);}await rm(scratch,{recursive:true,force:true});
}
