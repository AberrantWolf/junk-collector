// Isolated real-server inbox review acceptance with deterministic offline providers.
import {chromium} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,mkdir,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
import {createServer} from 'node:http';
import {intakeReviewJourney} from './intake-review-journey.mjs';
import {inboxCleanupJourney} from './inbox-cleanup-journey.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-intake-review-'));
const config=path.join(scratch,'collector.toml');
const password=randomBytes(24).toString('hex');
const origin='https://localhost:19873';
const releaseId='11111111-1111-4111-8111-111111111111';
const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64');
const discs=[];
const provider=createServer((request,response)=>{
 const url=new URL(request.url,'http://fixture.invalid');
 let body,type='application/json';
 if(url.pathname.startsWith('/ws/2/discid/')){
  const id=url.pathname.split('/').at(-1);if(!discs.includes(id))discs.push(id);
  body=Buffer.from(JSON.stringify({id,releases:[{id:releaseId,title:'Bulk resolved album','text-representation':{language:'en'},'artist-credit':[{name:'Review ensemble'}],'release-group':{id:'55555555-5555-4555-8555-555555555555'},media:discs.map((id,index)=>({position:index+1,discs:[{id}],tracks:[{position:1,title:`Disc ${index+1} opening`}]}))}]}));
 }else if(url.pathname===`/release/${releaseId}`){body=Buffer.from(JSON.stringify({release:`https://musicbrainz.org/release/${releaseId}`,images:[{id:'9001',front:true,back:false,approved:true,types:['Front'],comment:'Offline fixture',edit:1}]}));
 }else if(url.pathname.match(/\/9001(?:-250)?$/)){body=png;type='image/png';}
 else{response.writeHead(404);response.end();return;}
 response.writeHead(200,{'content-type':type,'content-length':body.length});response.end(body);
});
await new Promise((resolve,reject)=>{provider.once('error',reject);provider.listen(19874,'127.0.0.1',resolve);});
await mkdir(path.join(scratch,'inbox'));
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19871"\nprivate_bind = "127.0.0.1:19872"\nlan_bind = "127.0.0.1:19873"\nauthentication_origin = "${origin}"\nallowed_origins = ["${origin}"]\ntls_names = ["localhost", "127.0.0.1"]\nendpoints = ["${origin}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n[inboxes]\n"Optical shelf" = "${scratch}/inbox"\n[providers]\nmusicbrainz_user_agent = "collector-intake-review/0.1 (fixture@example.invalid)"\n`);
const binary=path.join(root,'target/debug/collector-server');
const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let logs='',server,browser,page;
async function startServer(){server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe'],env:{...process.env,JUNK_COLLECTOR_TEST_MUSICBRAINZ_ORIGIN:'http://127.0.0.1:19874/',JUNK_COLLECTOR_TEST_COVER_ART_ORIGIN:'http://127.0.0.1:19874/'}});server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
 for(let i=0;i<300;i++){try{if((await fetch('http://127.0.0.1:19871/healthz')).ok)return;}catch{}if(server.exitCode!==null)throw Error(logs);await new Promise(r=>setTimeout(r,100));}throw Error('Server readiness timed out');}
async function stopServer(){if(!server||server.exitCode!==null)return;server.kill('SIGINT');let timer;try{await Promise.race([new Promise(r=>server.once('exit',r)),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Server stop timed out')),10000);})]);}finally{clearTimeout(timer);if(server.exitCode===null)server.kill('SIGKILL');}}
try{
 await startServer();browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);page=await context.newPage();
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Review shelf');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>(await(await fetch('/api/v1/collection-search?limit=50')).json()).items.find(c=>c.name==='Review shelf').id);
 await page.goto(`${origin}/imports?collection=${collection}`);
 const cleanupFixture=await intakeReviewJourney(page,{scratch,startProviderFixture:()=>{},restartServer:async()=>{await stopServer();await startServer();}});
 await inboxCleanupJourney(page,{...cleanupFixture,scratch,restartServer:async()=>{await stopServer();await startServer();}});
 assert.deepEqual(errors,[]);console.log(`Chrome ${await browser.version()}; temporary artifacts: ${scratch}`);
}catch(error){console.error(logs);console.error('Browser state:',page?.url(),await page?.locator('body').innerText().catch(()=>''));await page?.screenshot({path:path.join(scratch,'failure.png'),fullPage:true}).catch(()=>{});throw error;
}finally{await browser?.close();await stopServer();provider.closeAllConnections();await new Promise(resolve=>provider.close(resolve));}
