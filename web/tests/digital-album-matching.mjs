// Offline provider fixtures; real audio, application jobs, storage and browser UI.
import {chromium,expect} from './support/browser.mjs';
import {spawn,spawnSync,stopProcess} from './support/runtime.mjs';
import {createServer} from 'node:http';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes,createHash,X509Certificate} from 'node:crypto';
const root=path.resolve(import.meta.dirname,'../..');const scratch=await mkdtemp(path.join(tmpdir(),'collector-digital-matching-'));const origin='https://localhost:19893';const password=randomBytes(24).toString('hex');
const release='00000000-0000-4000-8000-000000000090';const id=n=>`00000000-0000-4000-8000-${String(n).padStart(12,'0')}`;
const partial=true;const fingerprints=new Map();const requests=[];const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64');
const provider=createServer(async(req,res)=>{
 try{let body='';for await(const b of req)body+=b;requests.push(`${req.method} ${req.url} ${body}`);const url=new URL(req.url,'http://fixture');let result;
 if(url.pathname==='/v2/lookup'){const form=new URLSearchParams(body);assert.equal(form.get('client'),'fixture-key');const fp=form.get('fingerprint');const known={'67aa4ec8404218920890087a3b40dd3dc7d878c9665614801bc77f2ef87d48df':1,'ea75c59f8406db544ce308fdb1d25089af8352429ad5b6c483d1d9325c2172e8':2};const recording=known[createHash('sha256').update(fp).digest('hex')];assert.ok(recording,'unknown fingerprint');fingerprints.set(fp,recording);result={status:'ok',results:partial&&fingerprints.get(fp)===2?[]:[{score:.99,recordings:[{id:id(fingerprints.get(fp))}]}]};}
 else if(url.pathname.startsWith('/ws/2/release')){if(url.searchParams.has('recording'))result={'release-count':1,'release-offset':0,releases:[{id:release,title:'Matched album'}]};else result={id:release,title:'Matched album',disambiguation:'Steam',date:'2020-09-15',country:'XW','artist-credit':[{name:'Matched artist',artist:{id:id(99),name:'Matched artist'}}],media:[{id:id(98),position:1,format:'Digital Media','track-count':2,tracks:[{id:id(97),position:1,number:'1',title:'Matched first',length:12000,recording:{id:id(1),title:'Matched first',length:12000}},{id:id(96),position:2,number:'2',title:'Matched second',length:12000,recording:{id:id(2),title:'Matched second',length:12000}}]}]};}
 else if(url.pathname===`/release/${release}`)result={release:`https://musicbrainz.org/release/${release}`,images:[{id:1,front:true,back:false,approved:true,types:['Front'],comment:'Generated fixture',image:`https://coverartarchive.org/release/${release}/1`}]};
 else {res.writeHead(200,{'Content-Type':'image/png','Content-Length':png.length});res.end(req.method==='HEAD'?undefined:png);return;}
 const bytes=Buffer.from(JSON.stringify(result));res.writeHead(200,{'Content-Type':'application/json','Content-Length':bytes.length});res.end(bytes);
 }catch(e){res.writeHead(500);res.end(String(e));}
});await new Promise(r=>provider.listen(0,'127.0.0.1',r));const providerOrigin=`http://127.0.0.1:${provider.address().port}`;
const wav=path.join(scratch,'01.wav');
const decoded=spawnSync(process.env.COLLECTOR_TEST_FLAC&&process.env.COLLECTOR_TEST_FLAC!=='1'?process.env.COLLECTOR_TEST_FLAC:'flac',['--decode','--stdout','--silent',path.join(root,'crates/collector-store/tests/fixtures/digital-matching/01.flac')],{maxBuffer:8*1024*1024});
assert.equal(decoded.status,0,'Matching WAV fixture requires the independent FLAC decoder');await writeFile(wav,decoded.stdout);
const config=path.join(scratch,'collector.toml');await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19891"\nprivate_bind = "127.0.0.1:19892"\nlan_bind = "127.0.0.1:19893"\nlocal_playback_bind = "127.0.0.1:0"\nauthentication_origin = "${origin}"\nallowed_origins = ["${origin}"]\ntls_names = ["localhost", "127.0.0.1"]\nendpoints = ["${origin}"]\n[exposure]\npublic_curation = true\npublic_administration = true\n[providers]\nmusicbrainz_user_agent = "MatchingTests/1 (test@example.invalid)"\nacoustid_api_key = "fixture-key"\n`);
const binary=process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server');const setup=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let logs='';const server=spawn(binary,['--config',config,'serve'],{env:{...process.env,JUNK_COLLECTOR_TEST_MUSICBRAINZ_ORIGIN:providerOrigin,JUNK_COLLECTOR_TEST_ACOUSTID_ORIGIN:providerOrigin,JUNK_COLLECTOR_TEST_COVER_ART_ORIGIN:providerOrigin},stdio:['ignore','pipe','pipe']});server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
let browser,page,releaseDiscoveryResponse;
try{
 for(let i=0;i<300;i++){try{if((await fetch('http://127.0.0.1:19891/healthz')).ok)break;}catch{}if(server.exitCode!==null)throw Error(logs);await new Promise(r=>setTimeout(r,100));}
 const spki=createHash('sha256').update(new X509Certificate(await readFile(path.join(scratch,'state/tls/cert.pem'))).publicKey.export({type:'spki',format:'der'})).digest('base64');
 browser=await chromium.launch({mode:process.env.COLLECTOR_TEST_BROWSER||'chrome',headless:true,args:[`--ignore-certificate-errors-spki-list=${spki}`]});const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration'});await admin.getByLabel('Collection name').fill('Matching shelf');await admin.getByRole('combobox',{name:/Collection type/}).selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByText('Collection created.',{exact:false}).waitFor();await admin.getByRole('button',{name:'Close administration'}).click();
 const collection=await page.evaluate(async()=>{const result=await(await fetch('/api/v1/collection-search?limit=50')).json();return result.items.find(c=>c.name==='Matching shelf').id;});
 await page.goto(`${origin}/imports?collection=${collection}`);const imports=page.getByRole('region',{name:'Imports workspace'});await imports.getByLabel("Choose one package's files").setInputFiles([wav,path.join(root,'crates/collector-store/tests/fixtures/digital-matching/02.mp3')]);await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();await imports.getByLabel('Store',{exact:true}).fill('Private matching store');await imports.getByLabel('Order reference').fill('PRIVATE-MATCHING-ORDER');await imports.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));const intake=new URL(page.url()).searchParams.get('item');let item;
 await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);item=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:45000}).toBe('succeeded');
 await page.route('**/api/v1/items/'+item,route=>route.fulfill({status:503,json:{error:{code:'catalog_busy',message:'Metadata changed during read'}}}),{times:1});
 await page.route('**/api/v1/library-care/metadata?*',route=>route.fulfill({status:409,json:{error:{code:'revision_conflict',message:'Catalog changed'}}}),{times:1});
 await page.goto(`${origin}/library-care/metadata?collection=${collection}&item=${item}`);const matching=page.getByRole('region',{name:'Digital album matching'});await expect(matching.getByRole('button',{name:'Find matching albums',exact:true})).toBeEnabled();await expect(page.locator('.care-rows .care-row').first()).toBeVisible();// Hold an accepted response while inventory refresh supersedes its request scope.
 let acceptPost,releasePost;let refreshed=false,scopeReloaded=0;const accepted=new Promise(resolve=>acceptPost=resolve);const postGate=new Promise(resolve=>{releasePost=resolve;releaseDiscoveryResponse=resolve;});
 const matchUrl=`**/api/v1/items/${item}/digital-album-matches`;
 await page.route(matchUrl,async route=>{if(route.request().method()!=='POST'){if(refreshed)scopeReloaded++;await route.continue();return;}const response=await route.fetch();acceptPost();await postGate;await route.fulfill({response}).catch(()=>{});});
 await matching.evaluate(region=>{window.matchingAlerts=[];new MutationObserver(()=>{for(const alert of region.querySelectorAll('[role=alert]'))window.matchingAlerts.push(alert.textContent);}).observe(region,{subtree:true,childList:true,characterData:true});});
 await matching.getByRole('button',{name:'Find matching albums',exact:true}).click();await accepted;
 const itemUrl=`**/api/v1/items/${item}`;
 await page.route(itemUrl,async route=>{const response=await route.fetch();const data=await response.json();data.revision+=1;refreshed=true;await route.fulfill({json:data});});
 await page.getByRole('button',{name:'Refresh panel',exact:true}).click();await expect.poll(()=>refreshed,{timeout:15000}).toBe(true);await expect.poll(()=>scopeReloaded,{timeout:15000}).toBeGreaterThan(0);
 await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeVisible({timeout:90000});await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeDisabled();await expect(matching).toContainText('Find matching albums again');
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'stale matching notice overflows mobile');await page.screenshot({path:'/tmp/collector-stale-matching-mobile.png',fullPage:true});await page.setViewportSize({width:1440,height:1000});
 assert.deepEqual(await page.evaluate(()=>window.matchingAlerts),[],'obsolete discovery request surfaced an error after refresh');
 releasePost();await page.unroute(matchUrl);await page.unroute(itemUrl);

 // Reopening recovers the server-owned workflow without a second submission.
 const lostReceipt=`collector.digital-match.${await page.evaluate(async()=>(await(await fetch('/api/v1/me')).json()).user_id)}.${item}`;await expect.poll(()=>page.evaluate(key=>localStorage.getItem(key),lostReceipt)).toBeNull();
 await page.goto(`${origin}/`);await page.goto(`${origin}/library-care/metadata?collection=${collection}&item=${item}`);
 await expect(matching.getByRole('button',{name:'Edit track mapping'})).toBeEnabled({timeout:90000});await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeDisabled();await expect(matching).toContainText('1 of 2 files associated by audio recognition');await expect(matching.getByRole('img',{name:'Cover of Matched album'})).toBeVisible();
 const info=matching.getByText('Recognition can be incomplete.',{exact:false});
 await expect(info).toBeHidden();
 await expect(matching.locator('.digital-match-edition-badge')).toHaveText('Steam');
 await expect(matching.locator('.digital-match-region')).toContainText('Worldwide');
 const infoControl=matching.getByLabel('About audio recognition');
 // Native disclosure supports keyboard activation as well as touch/click.
 await infoControl.focus();await page.keyboard.press('Enter');await expect(info).toBeVisible();
 await matching.screenshot({path:'/tmp/collector-matching-info-desktop.png'});
 await page.setViewportSize({width:390,height:844});await info.scrollIntoViewIfNeeded();
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'recognition information overflows mobile');
 await matching.screenshot({path:'/tmp/collector-matching-info-mobile.png'});
 await infoControl.click();await expect(info).toBeHidden();await page.setViewportSize({width:1440,height:1000});
 await matching.getByText('Review proposed track mapping',{exact:true}).click();await expect(matching).toContainText('01.wav → disc 1, track 1: Matched first');
 const before=await page.evaluate(async id=>await(await fetch(`/api/v1/items/${id}`)).json(),item);assert.notEqual(before.title,'Matched album');assert.equal(fingerprints.size,2);assert.ok(requests.every(r=>!r.includes('PRIVATE-MATCHING-ORDER')&&!r.includes('Private matching store')));
 await page.setViewportSize({width:390,height:844});await matching.scrollIntoViewIfNeeded();assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'matching results overflow mobile');await mkdir(path.join(root,'web/test-results'),{recursive:true});await page.screenshot({path:path.join(root,'web/test-results/digital-matching-mobile.png'),fullPage:true});
 await matching.getByRole('button',{name:'Edit track mapping'}).click();
 const editor=matching.getByRole('group',{name:'Map every file to a release track'});
 await expect(editor.getByRole('combobox')).toHaveCount(2);
 await expect(editor.getByRole('checkbox')).toBeDisabled();
 await editor.getByRole('button',{name:'Assign in file order'}).click();
 await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeDisabled();
 await editor.getByRole('checkbox').check();
 // Duplicate assignments invalidate confirmation; correcting them requires fresh review.
 await editor.getByRole('combobox').nth(1).selectOption('1:1');
 await expect(editor.getByRole('checkbox')).not.toBeChecked();await expect(editor.getByRole('checkbox')).toBeDisabled();
 await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeDisabled();
 await editor.getByRole('combobox').nth(1).selectOption('1:2');
 await editor.getByRole('checkbox').check();
 await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeEnabled();
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'manual mapping overflows mobile');
 await page.screenshot({path:path.join(root,'web/test-results/digital-matching-manual-mobile.png'),fullPage:true});
 await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:path.join(root,'web/test-results/digital-matching-manual-desktop.png'),fullPage:true});await page.setViewportSize({width:390,height:844});
 const submission=page.waitForRequest(r=>r.method()==='POST'&&r.url().endsWith(`/items/${item}/metadata/resolutions`));
 await matching.getByRole('button',{name:'Use this album and track mapping'}).click();
 const submitted=(await submission).postDataJSON();assert.equal(submitted.lookup.assignments.length,2);

 await expect.poll(async()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()).title,item),{timeout:60000}).toBe('Matched album');
 // A visible preview must become saved, effective artwork in the same adoption workflow.
 await expect.poll(async()=>page.evaluate(async id=>{const m=await(await fetch(`/api/v1/items/${id}/metadata`)).json();return m.display.artwork;},item),{timeout:60000}).not.toBeNull();
 const adopted=await page.evaluate(async id=>await(await fetch(`/api/v1/items/${id}/metadata`)).json(),item);
 assert.equal(adopted.authority.selected_candidate.provider_record_id,release);
 assert.equal(adopted.authority.artwork_choice,null,'provider artwork must remain automatic');
 assert.equal(adopted.review_reasons.includes('missing_artwork'),false);
 const coverResponse=await page.request.get(`${origin}/api/v1/collections/${collection}/assets/${adopted.display.artwork}/thumbnail`);assert.equal(coverResponse.status(),200);
 const resolutions=await page.evaluate(async id=>await(await fetch(`/api/v1/items/${id}/metadata/resolutions`)).json(),item);
 assert.equal(resolutions[0].result.outcome,'candidates');assert.equal(resolutions[0].result.artwork_error,null);
 await expect(matching).toContainText('Find matching albums again',{timeout:15000});await expect(matching.getByRole('button',{name:'Use this album and track mapping'})).toBeDisabled();
 // A fresh search after acknowledgement admits exactly one new Job.
 await matching.getByRole('button',{name:'Find matching albums',exact:true}).click();
 await expect.poll(async()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/digital-album-matches`)).json()).jobs.length,item),{timeout:15000}).toBe(2);
 await expect(matching.getByRole('button',{name:'Edit track mapping'})).toBeEnabled({timeout:60000});await expect(matching).toContainText('1 of 2 files associated by audio recognition');
 assert.equal(await page.evaluate(key=>localStorage.getItem(key),lostReceipt),null);
 await page.goto(`${origin}/?collection=${collection}&item=${item}`);await page.getByRole('region',{name:'Album tracks'}).getByText('1. Matched first',{exact:true}).waitFor();await page.getByRole('region',{name:'Album tracks'}).getByText('2. Matched second',{exact:true}).waitFor();
 // Admit three albums through their real panels while the first POST is still unconfirmed.
 const newItems=[];
 for(let n=0;n<2;n++){
  await page.goto(`${origin}/imports?collection=${collection}`);
  const intakeForm=page.getByRole('region',{name:'Imports workspace'});
  await intakeForm.getByLabel("Choose one package's files").setInputFiles([wav,path.join(root,'crates/collector-store/tests/fixtures/digital-matching/02.mp3')]);
  await intakeForm.getByRole('heading',{name:'Review packages and groups'}).waitFor();
  await intakeForm.getByRole('button',{name:'Start import',exact:true}).click();await page.waitForURL(url=>url.searchParams.has('item'));
  const intakeId=new URL(page.url()).searchParams.get('item');let nextItem;
  await expect.poll(async()=>{const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intakeId);nextItem=members[0]?.import?.target_item_id;return members[0]?.import?.outcome;},{timeout:45000}).toBe('succeeded');
  newItems.push(nextItem);
 }
 assert.equal(new Set([item,...newItems]).size,3);
 const queueTimes=[];page.on('requestfinished',request=>{if(new URL(request.url()).pathname==='/api/v1/library-care/metadata')queueTimes.push(request.timing().responseEnd);});
 await page.goto(`${origin}/library-care/metadata?collection=${collection}&item=${item}&attention=all`);
 await expect(matching.getByRole('button',{name:'Find matching albums',exact:true})).toBeEnabled();
 let releaseAdmission,admissionSent;const heldAdmission=new Promise(resolve=>releaseAdmission=resolve);const admissionSeen=new Promise(resolve=>admissionSent=resolve);releaseDiscoveryResponse=releaseAdmission;
 await page.route(matchUrl,async route=>{if(route.request().method()!=='POST'){await route.continue();return;}admissionSent();await heldAdmission;await route.continue();});
 await matching.getByRole('button',{name:'Find matching albums',exact:true}).click();await admissionSeen;
 for(const nextItem of newItems){
  await page.route('**/api/v1/items/'+nextItem,route=>route.fulfill({status:503,json:{error:{code:'catalog_busy',message:'Metadata changed during read'}}}),{times:1});
  await page.getByRole('button',{name:'Back to metadata results',exact:true}).click();
  await page.locator(`[data-item-id="${nextItem}"]`).click();
  await expect(matching.getByRole('button',{name:'Find matching albums',exact:true})).toBeEnabled();
  await matching.getByRole('button',{name:'Find matching albums',exact:true}).click();
  await expect.poll(async()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/digital-album-matches`)).json()).jobs.length,nextItem)).toBe(1);
 }
 const admissionAck=page.waitForResponse(response=>response.request().method()==='POST'&&response.url().endsWith(`/items/${item}/digital-album-matches`));releaseAdmission();await admissionAck;await page.unroute(matchUrl);
 for(const [index,nextItem] of [item,...newItems].entries()){
  await expect.poll(async()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/digital-album-matches`)).json()).jobs[0]?.job.state,nextItem),{timeout:60000}).toBe('completed');
  const work=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/digital-album-matches`)).json()).jobs,nextItem);
  assert.equal(work.length,index===0?3:1,'each click admits exactly one durable album request');
 }
 const compact=await page.evaluate(async id=>await(await fetch(`/api/v1/library-care/metadata?collection_id=${id}&needs_attention=false`)).json(),collection);
 assert.ok(compact.items.length>=3);assert.ok(compact.items.every(row=>!('captures' in row)&&!('assets' in row)),'Metadata rows must not hydrate item graphs');
 assert.ok(queueTimes.length);console.log(`Metadata page requests during multi-album admission: ${queueTimes.map(n=>Math.round(n)).join(', ')} ms`);
 await expect(matching.getByRole('button',{name:'Edit track mapping'})).toBeEnabled({timeout:15000});await expect(matching.getByRole('button',{name:'Cancel album matching'})).toHaveCount(0);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'multi-album queue status overflows mobile');await page.screenshot({path:'/tmp/collector-metadata-queue-mobile.png',fullPage:true});
 assert.deepEqual(errors,[]);console.log('digital album matching: original WAV PCM and MP3 without configured encoders → provider discovery → reopened review → partial recognition → complete manual mapping → applied all tracks passed');
}catch(error){if(page)await page.screenshot({path:'/tmp/collector-digital-matching-failure.png',fullPage:true}).catch(()=>{});console.error(logs);throw error;}
finally{releaseDiscoveryResponse?.();if(browser)await browser.close();await stopProcess(server);provider.closeAllConnections();await new Promise(r=>provider.close(r));await rm(scratch,{recursive:true,force:true});}
