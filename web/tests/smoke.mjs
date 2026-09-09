import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-browser-'));
const password=randomBytes(24).toString('hex');
const localSetup=process.env.LOCAL_SETUP==='1';
const port=19843;
const config=path.join(scratch,'collector.toml');
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19841"\nprivate_bind = "127.0.0.1:19842"\nlan_bind = "127.0.0.1:${port}"\nauthentication_origin = "https://localhost:${port}"\nallowed_origins = ["https://localhost:${port}"]\ntls_names = ["localhost", "127.0.0.1"]\nendpoints = ["https://localhost:${port}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n`);
const binary=path.join(root,'target/debug/collector-server');
if(localSetup)await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19841"\nprivate_bind = "127.0.0.1:19842"\nlan_bind = "127.0.0.1:${port}"\nallowed_origins = []\n`);
const init=spawnSync(binary,['--config',config,localSetup?'setup':'init', '--username',localSetup?'collector':'owner'],{input:password,encoding:'utf8'});
assert.equal(init.status,0,init.stderr);
if(localSetup){const repeat=spawnSync(binary,['--config',config,'setup'],{input:'',encoding:'utf8'});assert.equal(repeat.status,0,repeat.stderr);}
const server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});
let logs='';server.stderr.on('data',d=>logs+=d);server.stdout.on('data',d=>logs+=d);
let browser; let page;
try{
 for(let i=0;i<100;i++){try{const response=await fetch('http://127.0.0.1:19841/healthz');if(response.ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 // Test-only self-signed HTTPS acceptance. Product clients never disable certificate validation.
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});
 page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 if(localSetup){const doctor=spawnSync(binary,['--config',config,'doctor'],{encoding:'utf8'});assert.equal(doctor.status,0,doctor.stderr);const features=await(await fetch('http://localhost:19841/api/v1/features')).json();assert.equal(features.administration,false);assert.equal(features.curation,false);}
 await page.goto(localSetup?'http://localhost:19842':`https://localhost:${port}`);
 await page.getByLabel('Password',{exact:true}).fill(password);
 await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Administration',exact:true}).waitFor();
 await page.getByRole('button',{name:'Administration',exact:true}).click();
 await page.getByLabel('Collection name').fill('Shelf & stereo');
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 await page.getByText('Collection created.',{exact:false}).waitFor();
 await page.getByRole('button',{name:'Close administration'}).click();
 await page.getByRole('button',{name:'Shelf & stereo',exact:true}).waitFor();
 await page.getByRole('button',{name:'Add item',exact:true}).click();
 await page.getByRole('combobox',{name:/Kind/}).selectOption('org.junk.music');
 await page.getByLabel('Title',{exact:true}).fill('A test pressing');
 await page.getByLabel('Artist / platform / edition',{exact:true}).fill('Test ensemble · Japan · CD');
 await page.getByLabel('Add an owned physical copy').check();
 await page.getByRole('button',{name:'Add',exact:true}).click();
 await page.getByText('Inventory saved.',{exact:false}).waitFor();
 // Synthetic one-pixel fixture; image decoding and real thumbnail bounds have separate Rust coverage.
 const png=Buffer.from(await page.evaluate(()=>{const c=document.createElement('canvas');c.width=160;c.height=160;const ctx=c.getContext('2d');ctx.fillStyle='#375e53';ctx.fillRect(0,0,160,160);ctx.fillStyle='#e5d69a';ctx.beginPath();ctx.arc(80,80,52,0,Math.PI*2);ctx.fill();ctx.fillStyle='#375e53';ctx.beginPath();ctx.arc(80,80,12,0,Math.PI*2);ctx.fill();return c.toDataURL('image/png').split(',')[1];}),'base64');
 await page.getByLabel('PNG, JPEG, or WebP').setInputFiles({name:'cover.png',mimeType:'image/png',buffer:png});
 await page.getByRole('button',{name:'Attach cover',exact:true}).click();
 await page.getByText('Original artwork and thumbnail saved.').waitFor();
 await page.getByRole('button',{name:'Close curator'}).click();
 await page.getByRole('button',{name:/A test pressing/}).waitFor();
 assert.equal(await page.locator('.collection-row img').count(),1);
 await page.getByRole('searchbox').fill('pressing');
 await page.waitForTimeout(350);
 assert.equal(await page.locator('.collection-row').count(),1);
 // Standard browser WebAuthn, with a virtual authenticator instead of a physical password manager.
 if(!localSetup){
 const cdp=await context.newCDPSession(page);await cdp.send('WebAuthn.enable');
 await cdp.send('WebAuthn.addVirtualAuthenticator',{options:{protocol:'ctap2',transport:'internal',hasResidentKey:true,hasUserVerification:true,isUserVerified:true,automaticPresenceSimulation:true}});
 await page.getByRole('button',{name:'Add a passkey',exact:true}).click();
 await page.getByText('Passkey registered.',{exact:false}).waitFor();
 await page.getByRole('button',{name:'Sign out',exact:true}).click();
 await page.getByRole('button',{name:'Sign in with a passkey',exact:true}).click();
 await page.getByRole('button',{name:'Administration',exact:true}).waitFor();
 }
 await page.screenshot({path:path.join(root,'web/test-results/browser-desktop.png'),fullPage:true});
 await page.setViewportSize({width:390,height:844});
 await page.screenshot({path:path.join(root,'web/test-results/browser-mobile.png'),fullPage:true});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile horizontal overflow');
 assert.deepEqual(errors,[]);
 console.log(localSetup?'Local setup smoke passed: repaired origins, repeat setup, live doctor, custom default profile, cookie login, private management, inventory and artwork.':'Browser smoke passed: session login, management bundles, inventory, artwork rows, search, WebAuthn registration/login, mobile layout.');
}catch(error){console.error(logs);if(page){console.error((await page.locator('body').innerText()).slice(0,6000));await page.screenshot({path:path.join(root,'web/test-results/failure.png'),fullPage:true});}throw error;}finally{if(browser)await browser.close();server.kill('SIGINT');await new Promise(resolve=>server.once('exit',resolve));await rm(scratch,{recursive:true,force:true});}
