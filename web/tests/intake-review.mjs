import {stopProcess} from './support/runtime.mjs';
// Isolated real-server inbox review acceptance with deterministic offline providers.
import {chromium} from './support/browser.mjs';
import {spawn,spawnSync} from './support/runtime.mjs';
import {mkdtemp,mkdir,writeFile,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
import {createServer} from 'node:http';
import {intakeReviewJourney} from './intake-review-journey.mjs';
import {inboxCleanupJourney} from './inbox-cleanup-journey.mjs';
import {ingestProgressJourney} from './ingest-progress-journey.mjs';
import {automaticAlbumGroupingJourney} from './automatic-album-grouping-journey.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-intake-review-'));
const config=path.join(scratch,'collector.toml');
const password=randomBytes(24).toString('hex');
const origin='https://localhost:19873';
const releaseId='11111111-1111-4111-8111-111111111111';
const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64');
// Fixed one-track TOC vectors: 225–228 audio sectors, offset 150.
const automaticDiscs=['KMo70rqGAUyPNJCab5I0W0aV7zI-','o5z.aMadhkEVfFTU8yGG4fLXMq0-','k6fu.rA16ATyJiYpOkKMnCFDzc4-','ByWmT_KjfsCRLUJWcvw.rvMi8x8-'];
const discs=process.env.AUTO_GROUPING_ONLY?[...automaticDiscs]:[];
let holdMusicBrainz=false,conflictingProgress=false;const pendingProviderResponses=new Set();
function holdProvider(){holdMusicBrainz=true;conflictingProgress=true;return ()=>{holdMusicBrainz=false;for(const respond of pendingProviderResponses)respond();pendingProviderResponses.clear();};}
const provider=createServer((request,response)=>{
 const url=new URL(request.url,'http://fixture.invalid');
 let body,type='application/json';
 if(url.pathname.startsWith('/ws/2/discid/')){
  const id=url.pathname.split('/').at(-1);if(!discs.includes(id))discs.push(id);
  const matched={id:releaseId,title:'Bulk resolved album','text-representation':{language:'eng'},'artist-credit':[{name:'Review ensemble',artist:{id:'44444444-4444-4444-8444-444444444444',name:'Review ensemble'}}],'release-group':{id:'55555555-5555-4555-8555-555555555555',title:'Fixture release group'},media:discs.map((id,index)=>({id:`77777777-7777-4777-8777-${String(index+1).padStart(12,'0')}`,position:index+1,'track-count':1,discs:[{id,'offset-count':1,offsets:[150],sectors:300}],tracks:[{id:`88888888-8888-4888-8888-${String(index+1).padStart(12,'0')}`,position:1,number:'1',title:`Disc ${index+1} opening`}]}))};
  // Progress acceptance needs reviewed grouping even when a safe subset of
  // distinct discs finishes before the duplicate-position entries do.
  const alternatives=process.env.GROUPING_AMBIGUITY?[matched,{...matched,id:'22222222-2222-4222-8222-222222222222',title:'Another possible album','release-group':{id:'66666666-6666-4666-8666-666666666666',title:'Another possible release group'}}]:conflictingProgress?[matched,{...matched,id:'33333333-3333-4333-8333-333333333333',media:matched.media.map((medium,index)=>({...medium,discs:matched.media[matched.media.length-1-index].discs}))}]:[matched];
  body=Buffer.from(JSON.stringify({id,releases:alternatives}));
 }else if(url.pathname.match(/^\/release\/[^/]+$/)){body=Buffer.from(JSON.stringify({release:`https://musicbrainz.org/release/${releaseId}`,images:[{id:'9001',front:true,back:false,approved:true,types:['Front'],comment:'Offline fixture',edit:1}]}));
 }else if(url.pathname.match(/\/9001(?:-250)?$/)){body=png;type='image/png';}
 else{response.writeHead(404);response.end();return;}
 const respond=()=>{response.writeHead(200,{'content-type':type,'content-length':body.length});response.end(body);};
 if(holdMusicBrainz&&url.pathname.startsWith('/ws/2/discid/')){pendingProviderResponses.add(respond);response.once('close',()=>pendingProviderResponses.delete(respond));}else respond();
});
await new Promise((resolve,reject)=>{provider.once('error',reject);provider.listen(19874,'127.0.0.1',resolve);});
await mkdir(path.join(scratch,'inbox'));
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19871"\nprivate_bind = "127.0.0.1:19872"\nlan_bind = "127.0.0.1:19873"\nlocal_playback_bind = "127.0.0.1:0"\nauthentication_origin = "${origin}"\nallowed_origins = ["${origin}"]\ntls_names = ["localhost", "127.0.0.1"]\nendpoints = ["${origin}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n[inboxes]\n"Optical shelf" = "${scratch}/inbox"\n[providers]\nmusicbrainz_user_agent = "collector-intake-review/0.1 (fixture@example.invalid)"\n`);
if(process.env.INGEST_WORKERS)await writeFile(config,(await readFile(config,'utf8'))+`\n[jobs]\nprocessing_workers = ${Number(process.env.INGEST_WORKERS)}\ncopy_workers = 1\n`);
const binary=(process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server'));
const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let logs='',server,browser,page;
async function startServer(){server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe'],env:{...process.env,JUNK_COLLECTOR_TEST_ACCURATERIP_ORIGIN:'http://127.0.0.1:19874/',JUNK_COLLECTOR_TEST_MUSICBRAINZ_ORIGIN:'http://127.0.0.1:19874/',JUNK_COLLECTOR_TEST_COVER_ART_ORIGIN:'http://127.0.0.1:19874/'}});server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
 for(let i=0;i<300;i++){try{if((await fetch('http://127.0.0.1:19871/healthz')).ok)return;}catch{}if(server.exitCode!==null)throw Error(logs);await new Promise(r=>setTimeout(r,100));}throw Error('Server readiness timed out');}
async function stopServer(){await stopProcess(server);}
try{
 await startServer();browser=await chromium.launch({headless:true});
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);page=await context.newPage();
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});
 await admin.getByLabel('Collection name').fill('Review shelf');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>(await(await fetch('/api/v1/collection-search?limit=50')).json()).items.find(c=>c.name==='Review shelf').id);
 await page.goto(`${origin}/imports?collection=${collection}`);
 if(process.env.AUTO_GROUPING_ONLY){
 await automaticAlbumGroupingJourney(page,{scratch,collectionId:collection,restartServer:async()=>{await stopServer();await startServer();}});
 }else{
 if(!process.env.INGEST_PROGRESS_ONLY){
 const cleanupFixture=await intakeReviewJourney(page,{scratch,startProviderFixture:()=>{},restartServer:async()=>{await stopServer();await startServer();}});
 await inboxCleanupJourney(page,{...cleanupFixture,scratch,restartServer:async()=>{await stopServer();await startServer();}});
 }
 await ingestProgressJourney(page,{scratch,holdProvider,collectionId:collection});
 }
 assert.deepEqual(errors,[]);console.log(`Chrome ${await browser.version()}; temporary artifacts: ${scratch}`);
}catch(error){console.error(logs);console.error('Browser state:',page?.url(),await page?.locator('body').innerText({timeout:2000}).catch(()=>''));await page?.screenshot({path:path.join(scratch,'failure.png'),fullPage:true}).catch(()=>{});throw error;
}finally{await browser?.close();await stopServer();provider.closeAllConnections();await new Promise(resolve=>provider.close(resolve));}
