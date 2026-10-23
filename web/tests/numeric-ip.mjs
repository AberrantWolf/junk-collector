// Actual numeric-IP HTTPS startup, password login, capability guidance and recovery.
import {chromium,expect} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import https from 'node:https';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-ip-origin-'));
const config=path.join(scratch,'collector.toml'),origin='https://127.0.0.1:19663';
const password=randomBytes(24).toString('hex');
const artifact=path.join(tmpdir(),'collector-ip-passkeys.png');
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19661"\nprivate_bind = "127.0.0.1:19662"\nlan_bind = "127.0.0.1:19663"\nlocal_playback_bind = "127.0.0.1:0"\nauthentication_origin = "${origin}"\nallowed_origins = ["${origin}"]\ntls_names = ["127.0.0.1"]\n`);
const binary=path.join(root,'target/debug/collector-server');
let browser,page,server,logs='';
try{
 const setup=spawnSync(binary,['--config',config,'setup'],{input:password,encoding:'utf8',timeout:30000});assert.equal(setup.status,0,setup.stderr);
 const doctor=spawnSync(binary,['--config',config,'doctor'],{encoding:'utf8',timeout:30000});assert.equal(doctor.status,0,doctor.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
 for(let i=0;i<200;i++){
  try{if((await fetch('http://127.0.0.1:19661/healthz')).ok)break;}catch{}
  if(server.exitCode!==null)throw Error(logs);if(i===199)throw Error('Numeric-IP server did not start');await new Promise(resolve=>setTimeout(resolve,100));
 }
 const ca=await readFile(path.join(scratch,'state/tls/cert.pem'));
 // Independently validate the actual IP SAN and certificate before the isolated
 // browser context accepts this fixture's self-signed certificate.
 await new Promise((resolve,reject)=>{const req=https.get(origin+'/api/v1/identity',{ca},response=>{response.resume();response.on('end',()=>{try{assert.equal(response.statusCode,200);resolve();}catch(error){reject(error);}});});req.setTimeout(5000,()=>req.destroy(Error('TLS timed out')));req.on('error',reject);});
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const context=await browser.newContext({ignoreHTTPSErrors:true});context.setDefaultTimeout(30000);
 page=await context.newPage();const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('heading',{name:'Library',exact:true}).waitFor();
 const features=await(await page.request.get(origin+'/api/v1/features')).json();assert.equal(features.passkeys,false);assert.equal(features.passkeys_unavailable_reason,'dns_name_required');
 await page.getByRole('button',{name:'Account & security',exact:true}).click();const security=page.getByRole('dialog',{name:'Account & security'});
 await security.getByRole('button',{name:'Passkeys',exact:true}).click();
 await expect(security.getByText('Password sign-in and recovery work at this IP address.',{exact:false})).toBeVisible();assert.equal(await security.getByRole('button',{name:'Add a passkey',exact:true}).count(),0);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.screenshot({path:artifact,fullPage:true});
 await security.getByRole('button',{name:'Recovery',exact:true}).click();await security.getByRole('button',{name:'Generate replacement codes',exact:true}).click();
 const code=await security.getByRole('list',{name:'Recovery codes'}).locator('code').first().textContent();assert.ok(code);
 page.once('dialog',dialog=>dialog.accept());await security.getByRole('button',{name:'Close account and security',exact:true}).click();
 // A recovery request must also work through this numeric-IP origin, with the
 // usual fresh password/session, rather than merely displaying capability text.
 await page.setViewportSize({width:1440,height:1000});await page.getByRole('button',{name:'Sign out',exact:true}).click();
 await page.getByRole('button',{name:'Use a recovery code',exact:true}).click();await page.getByLabel('Recovery code',{exact:true}).fill(code);await page.getByLabel('New password',{exact:true}).fill(randomBytes(24).toString('hex'));
 await page.getByRole('button',{name:'Use recovery code',exact:true}).click();await page.getByRole('heading',{name:'Library',exact:true}).waitFor();
 assert.deepEqual(errors,[]);console.log(`Numeric-IP browser passed (Chrome ${await browser.version()}): actual TLS/IP identity, setup/doctor/startup, password sign-in, typed passkey guidance, mobile layout and recovery-code sign-in. Screenshot: ${artifact}`);
}catch(error){console.error(logs);await page?.screenshot({path:artifact,fullPage:true}).catch(()=>{});throw error;}
finally{
 await browser?.close();
 if(server){const exit=new Promise(resolve=>server.once('exit',resolve));if(server.exitCode===null){server.kill('SIGINT');await Promise.race([exit,new Promise(resolve=>setTimeout(resolve,5000))]);}if(server.exitCode===null){server.kill('SIGKILL');await exit;}}
 await rm(scratch,{recursive:true,force:true});
}
