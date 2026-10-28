import {waitForServer} from './support/runtime.mjs';
import {stopProcess} from './support/runtime.mjs';
import {clickAction} from './actions.mjs';
// FX-01 real-server navigation regression. All archive/state roots are temporary.
import {chromium,expect} from './support/browser.mjs';
import {spawn,spawnSync} from './support/runtime.mjs';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-browse-'));
const config=path.join(scratch,'collector.toml');
const binary=(process.env.COLLECTOR_SERVER_BINARY||path.join(root,'target/debug/collector-server'));
const password=randomBytes(24).toString('hex');
const origin='http://localhost:19942';
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19941"\nprivate_bind = "127.0.0.1:19942"\nlan_bind = "127.0.0.1:19943"\nlocal_playback_bind = "127.0.0.1:0"\nallowed_origins = []\n`);
 const init=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(init.status,0,init.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',v=>logs+=v);server.stdout.on('data',v=>logs+=v);
 await waitForServer(origin+'/healthz',server,{logs:()=>logs});
 browser=await chromium.launch({headless:true});
 const context=await browser.newContext({viewport:{width:1440,height:1000}});page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(15000);
 await page.goto(origin);for(const name of ['app.js','app.css','curation.js'])assert.equal(await page.evaluate(async name=>await(await fetch('/'+name)).text(),name),await readFile(path.join(root,'web/dist',name),'utf8'),'server contains stale '+name);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 const admin=page.getByRole('dialog');await admin.getByLabel('Collection name').fill('Browse music');await admin.getByLabel('Collection type').selectOption('org.junk.music');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByText('Collection created.',{exact:false}).waitFor();
 await admin.getByLabel('Collection name').fill('Empty games');await admin.getByLabel('Collection type').selectOption('org.junk.games');await admin.getByRole('button',{name:'Create collection',exact:true}).click();await admin.getByLabel('Collection name').evaluate(async input=>{for(let n=0;input.value&&n<100;n++)await new Promise(r=>setTimeout(r,30));});
 await page.getByRole('button',{name:'Close administration'}).click();await page.locator('.collection-scope summary').click();await page.getByRole('button',{name:'Browse music',exact:true}).waitFor();
 const seed=await page.evaluate(async()=>{
  const collections=(await(await fetch('/api/v1/collection-search?limit=100')).json()).items;const music=collections.find(c=>c.name==='Browse music');const ids=[];
  for(let n=0;n<62;n++){
   const id=crypto.randomUUID();const response=await fetch('/api/v1/items/'+id,{method:'PUT',headers:{'Content-Type':'application/json','Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({expected_revision:0,edit:{collection_id:music.id,title:'Album '+String(n).padStart(3,'0'),subtitle:'Browser fixture',identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{artist:n%2?'Zebra':'Alpha'}},identity:'unidentified',translations:[],identifiers:[],wishlist:n%2===0}})});if(!response.ok)throw new Error(await response.text());ids.push(id);
  }
  const response=await fetch('/api/v1/imports',{method:'POST',headers:{'Content-Type':'application/json','Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({collection_id:music.id,package_name:'Browse return work',files:[{name:'disc.bin',bytes:4,sha256:null}]})});if(!response.ok)throw new Error(await response.text());
  return {music:music.id,ids,importId:(await response.json()).id};
 });
 // Changes prepare summaries before any Library read for these new entries.
 // Inspect only the disposable fixture's derived index, without calling Library.
 const prepared=spawnSync('python3',['-c',`import sqlite3,time,sys
c=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True,timeout=1)
end=time.monotonic()+15
while True:
 count=c.execute('SELECT COUNT(*) FROM library_entries c JOIN library_entry_versions v ON v.item_id=c.item_id AND v.token=c.token JOIN items i ON i.id=c.item_id WHERE i.collection_id=?',(sys.argv[2],)).fetchone()[0]
 if count==62: print(count);break
 if time.monotonic()>end: raise RuntimeError('Background Library projection did not settle: '+str(count))
 time.sleep(.05)
`,path.join(scratch,'state/catalog.sqlite'),seed.music],{encoding:'utf8',timeout:18000});
 assert.equal(prepared.status,0,prepared.stderr);assert.equal(prepared.stdout.trim(),'62');
 // A pending response must render basic rows immediately and poll to readiness.
 let holdSummary=true;const pendingSummaryRoute=async route=>{const response=await route.fetch();if(!holdSummary){await route.fulfill({response});return;}const body=await response.json();await route.fulfill({response,json:{...body,summary_state:'pending',items:body.items.map(item=>({...item,summary_state:'pending',artwork:null,accuraterip:null,album_playback:null,can_edit:false}))}});};
 await page.route('**/api/v1/library?*',pendingSummaryRoute);
 await page.reload();await page.locator('.collection-row').first().waitFor();
 await expect(page.getByRole('status').filter({hasText:'Updating library summaries'})).toBeVisible();
 await expect(page.locator('.collection-row').first().getByLabel('Artwork summary updating')).toBeVisible();
 assert.equal(await page.getByText('No originals',{exact:true}).count(),0,'pending summary claimed missing originals');
 if(process.env.BROWSE_SCREENSHOTS){await mkdir(process.env.BROWSE_SCREENSHOTS,{recursive:true});await page.screenshot({path:path.join(process.env.BROWSE_SCREENSHOTS,'desktop-summary-pending.png')});await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);await page.screenshot({path:path.join(process.env.BROWSE_SCREENSHOTS,'mobile-summary-pending.png')});await page.setViewportSize({width:1440,height:1000});}
 holdSummary=false;
 await expect(page.locator('.collection-row').first()).toContainText('Alpha');
 await expect(page.getByRole('status').filter({hasText:'Updating library summaries'})).toHaveCount(0);
 await page.unroute('**/api/v1/library?*',pendingSummaryRoute);
 await page.waitForFunction(()=>document.querySelectorAll('.collection-row').length===50);
 assert.equal(await page.locator('.collection-row').count(),50);assert.match(await page.locator('.collection-row').first().innerText(),/Album 000/);
 const nextPageRequests=[];page.on('request',request=>{if(new URL(request.url()).pathname==='/api/v1/library')nextPageRequests.push(request.url());});
 await page.getByRole('button',{name:'Load more',exact:true}).click();await page.locator(`[data-item-id="${seed.ids[61]}"]`).waitFor();assert.equal(await page.locator('.collection-row').count(),62);assert.equal(nextPageRequests.length,1,'loading another page refetched earlier pages');assert.ok(new URL(nextPageRequests[0]).searchParams.has('cursor'));
 const row=page.locator(`[data-item-id="${seed.ids[55]}"]`);await row.scrollIntoViewIfNeeded();const scroll=await page.evaluate(()=>scrollY);await row.click();
 await page.getByLabel('Album 055 details',{exact:true}).waitFor();assert.ok(page.url().includes(seed.ids[55]));
 const bounds=await page.locator('.detail-view').boundingBox();assert.ok(bounds.y>=0&&bounds.y<1000,'desktop detail did not remain visible');
 await page.getByRole('button',{name:'Back to results',exact:true}).click();await page.waitForFunction(()=>!new URL(location.href).searchParams.has('item'));
 assert.equal(await page.locator('.collection-row').count(),62);await page.waitForFunction(y=>Math.abs(scrollY-y)<8,scroll);assert.equal(await page.evaluate(()=>document.activeElement?.dataset.itemId),seed.ids[55]);
 await page.goForward();await page.getByLabel('Album 055 details',{exact:true}).waitFor();await page.reload();await page.getByLabel('Album 055 details',{exact:true}).waitFor();
 // Nested work navigation must not change the explicit Library return target.
 async function openWork(){
  await clickAction(page,'Edit / attach files');const curator=page.getByRole('dialog');await curator.getByRole('button',{name:'Advanced evidence',exact:true}).click();await curator.getByRole('button',{name:'Open work & history for this entry',exact:true}).click();
  const work=page.getByRole('dialog',{name:'Work & history'});await work.waitFor();await work.getByRole('button',{name:'Show work for all entries',exact:true}).click();await work.getByLabel('Search work',{exact:true}).fill('Browse return work');await work.locator(`[data-work-open="import:${seed.importId}"]`).click();await work.getByRole('region',{name:'Selected work'}).waitFor();return work;
 }
 const libraryTiming=await page.evaluate(async collection=>{
  const runs=[];
  for(let run=0;run<2;run++){
   const start=performance.now();const response=await fetch('/api/v1/library?'+new URLSearchParams({collection_id:collection,limit:'200'}),{cache:'no-store'});
   if(!response.ok)throw new Error('Library summary request failed: '+response.status);
   const text=await response.text();const body=JSON.parse(text);
   if(body.items.length!==62||body.items.some(item=>'captures' in item||'domain' in item||'effective_metadata' in item))throw new Error('Library leaked full item graphs or lost rows');
   runs.push({ms:Math.round(performance.now()-start),bytes:new TextEncoder().encode(text).length,count:body.items.length});
  }
  return runs;
 },seed.music);
 console.log('Compact Library repeat loads:',JSON.stringify(libraryTiming));
 const work=await openWork();
 await page.goBack();await work.getByRole('button',{name:'Refresh work',exact:true}).waitFor();assert.equal(new URL(page.url()).searchParams.get('work'),'list');
 // Restore through popstate after the modal has actually unmounted, too.
 await work.getByRole('button',{name:'Close work history'}).click();await work.waitFor({state:'detached'});
 await page.goForward();await work.getByRole('region',{name:'Selected work'}).getByRole('heading',{name:'Browse return work',exact:true}).waitFor();assert.equal(new URL(page.url()).searchParams.get('work'),`import:${seed.importId}`);
 // Leaving the document and intentionally going Back must restore the work detail.
 await page.goto(origin+'/?item='+seed.ids[54]);await page.getByLabel('Album 054 details',{exact:true}).waitFor();await page.goBack();await work.getByRole('region',{name:'Selected work'}).getByRole('heading',{name:'Browse return work',exact:true}).waitFor();
 await work.getByRole('button',{name:'Close work history'}).click();
 const reopenedWork=await openWork();await reopenedWork.getByRole('button',{name:'Back to history',exact:true}).click();await reopenedWork.getByRole('button',{name:'Refresh work',exact:true}).waitFor();await reopenedWork.locator(`[data-work-open="import:${seed.importId}"]`).click();await reopenedWork.getByRole('region',{name:'Selected work'}).waitFor();await reopenedWork.getByRole('button',{name:'Close work history'}).click();
 await page.getByRole('button',{name:'Back to results',exact:true}).click();await page.waitForFunction(()=>!new URL(location.href).searchParams.has('item')&&!new URL(location.href).searchParams.has('work'));
 assert.equal(await work.count(),0,'Back to results reopened Work & history');assert.equal(await page.locator('.collection-row').count(),62);await page.waitForFunction(y=>Math.abs(scrollY-y)<8,scroll);assert.equal(await page.evaluate(()=>document.activeElement?.dataset.itemId),seed.ids[55]);
 await row.click();await page.getByLabel('Album 055 details',{exact:true}).waitFor();
 await clickAction(page,'Edit / attach files');const editor=page.getByRole('dialog');await editor.getByLabel('Title',{exact:true}).fill('Edited album 055');await editor.getByRole('button',{name:'Save details',exact:true}).click();await editor.getByText('Entry saved.',{exact:false}).waitFor();await editor.getByRole('button',{name:'Artwork',exact:true}).click();
 await page.route('**/assets/*/thumbnail*',route=>route.fulfill({status:503,body:'Temporary image failure'}));
 await editor.getByLabel('Artwork image').setInputFiles({name:'cover.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64')});await editor.getByRole('button',{name:'Attach image',exact:true}).click();await editor.getByText('Artwork and photos updated.',{exact:true}).waitFor();
 await page.getByRole('button',{name:'Close curator'}).click();await page.getByRole('button',{name:'Retry artwork',exact:true}).waitFor();assert.equal(await page.locator('.detail .artwork').getByLabel('No artwork attached').count(),0);await page.unroute('**/assets/*/thumbnail*');await page.getByRole('button',{name:'Retry artwork',exact:true}).click();await page.locator('.detail .artwork img').waitFor();
 await page.getByLabel('Edited album 055 details',{exact:true}).waitFor();
 page.once('dialog',dialog=>dialog.accept());await clickAction(page,'Remove item');
 await page.getByRole('button',{name:'Restore entry',exact:true,includeHidden:true}).waitFor({state:'attached'});assert.equal(await page.getByLabel('Show',{exact:true}).inputValue(),'removed');
 await page.locator('.collection-row').first().waitFor();assert.equal(await page.locator('.collection-row').count(),1);await page.reload();await page.getByRole('button',{name:'Restore entry',exact:true,includeHidden:true}).waitFor({state:'attached'});
 await clickAction(page,'Restore entry');await page.getByRole('button',{name:'Remove item',exact:true,includeHidden:true}).waitFor({state:'attached'});assert.equal(await page.getByLabel('Show',{exact:true}).inputValue(),'all');
 await page.getByRole('button',{name:'Back to results',exact:true}).click();await page.waitForFunction(()=>!new URL(location.href).searchParams.has('item'));await page.getByRole('button',{name:'Load more',exact:true}).click();await page.locator(`[data-item-id="${seed.ids[55]}"]`).waitFor();assert.match(await page.locator(`[data-item-id="${seed.ids[55]}"]`).innerText(),/Edited album 055/);
 await page.getByLabel('Show',{exact:true}).selectOption('wishlist');await page.waitForFunction(()=>document.querySelectorAll('.collection-row').length===31);await page.reload();await page.locator('.collection-row').first().waitFor();assert.equal(await page.getByLabel('Show',{exact:true}).inputValue(),'wishlist');
 // Simulate a sparse server page with a real scoped continuation: later matches
 // must appear without asking the user to scrub through healthy albums.
 await page.route('**/api/v1/library?*',async route=>{
  const response=await route.fetch();const page=await response.json();assert.ok(page.next_cursor);
  await route.fulfill({response,json:{...page,items:[]}});
 },{times:1});
 await page.getByLabel('Show',{exact:true}).selectOption('album_playback_needs_attention');
 await page.waitForFunction(()=>new URL(location.href).searchParams.get('filter')==='album_playback_needs_attention'&&document.querySelectorAll('.collection-row').length===12);
 await expect(page.locator('.collection-row').first().getByText('Album playback needs attention',{exact:true})).toBeVisible();
 await page.getByLabel('Show',{exact:true}).selectOption('all');await page.waitForFunction(()=>document.querySelectorAll('.collection-row').length===50);
 // An empty filtered page with a failed summary is still unfinished work.
 await page.route('**/api/v1/library?*',async route=>{const response=await route.fetch(),body=await response.json();await route.fulfill({response,json:{...body,summary_state:'retrying',items:[],next_cursor:null}});},{times:1});
 await page.getByLabel('Show',{exact:true}).selectOption('album_playback_needs_attention');
 await expect(page.getByRole('alert').filter({hasText:'Some summaries could not be updated'})).toBeVisible();
 assert.equal(await page.getByRole('heading',{name:'No matching entries',exact:true}).count(),0,'unfinished filtered page appeared empty');
 await page.waitForFunction(()=>document.querySelectorAll('.collection-row').length===50||document.querySelectorAll('.collection-row').length===12);
 await expect(page.locator('.collection-row').first().getByText('Album playback needs attention',{exact:true})).toBeVisible();
 await page.reload();await expect(page.getByLabel('Show',{exact:true})).toHaveValue('album_playback_needs_attention');
 await page.locator('.collection-row').first().click();
 await expect(page.locator('.album-playback-notice')).toContainText('No recorded audio tracks are available');
 await page.getByRole('button',{name:'Back to results',exact:true}).click();
 await page.getByRole('button',{name:'Clear filters',exact:true}).click();await page.getByLabel('Sort by').selectOption('recently_added');await page.waitForFunction(()=>document.querySelector('.collection-row')?.textContent.includes('Album 061'));
 await page.getByLabel('Sort by').selectOption('artist');await page.waitForFunction(()=>document.querySelector('.collection-row')?.textContent.includes('Alpha'));await page.getByLabel('Sort by').selectOption('title');
 await page.getByRole('searchbox',{name:'Search collection'}).fill('Album 001');await page.waitForFunction(()=>document.querySelectorAll('.collection-row').length===1);await page.locator('.collection-row').click();await page.getByLabel('Album 001 details',{exact:true}).waitFor();
 await page.locator('.collection-scope summary').click();await page.getByRole('button',{name:'Empty games',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('.detail'));assert.ok(!new URL(page.url()).searchParams.has('item'));
 await page.getByRole('searchbox',{name:'Search collection'}).fill('');await page.locator('.collection-scope summary').click();await page.getByRole('button',{name:'Browse music',exact:true}).click();await page.locator('.collection-row').first().waitFor();
 await page.setViewportSize({width:390,height:844});await page.locator('.collection-row').first().click();await page.getByLabel('Album 000 details',{exact:true}).waitFor();assert.equal(await page.locator('.entry-list').isVisible(),false);assert.ok((await page.locator('.detail-view').boundingBox()).y<150);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
 await page.keyboard.press('Escape');await page.locator('.collection-row').first().waitFor();await page.waitForFunction(id=>document.activeElement?.dataset.itemId===id,seed.ids[0],{timeout:1000});assert.equal(await page.evaluate(()=>document.activeElement?.dataset.itemId),seed.ids[0]);await page.keyboard.press('Enter');await page.getByLabel('Album 000 details',{exact:true}).waitFor();
 if(process.env.BROWSE_SCREENSHOTS){await mkdir(process.env.BROWSE_SCREENSHOTS,{recursive:true});await page.screenshot({path:path.join(process.env.BROWSE_SCREENSHOTS,'mobile-detail.png')});}
 await page.getByRole('button',{name:'Back to results',exact:true}).click();await page.setViewportSize({width:1440,height:1000});
 // Late evidence, explicit read failure, and retry use the real UI with controlled transport faults.
 let slowStarted;const started=new Promise(r=>slowStarted=r);
 await page.route('**/api/v1/items/*/evidence',async route=>{const first=route.request().url().includes(seed.ids[0]);if(first){slowStarted();await new Promise(r=>setTimeout(r,1000));}await route.fulfill({json:[{id:first?'old':'new',kind:'integrity_verification',outcome:first?'OLD EVIDENCE':'CURRENT EVIDENCE'}]}).catch(()=>{});});
 await page.locator(`[data-item-id="${seed.ids[0]}"]`).click();await started;await page.locator(`[data-item-id="${seed.ids[1]}"]`).click();await page.getByText('Album details, evidence & files',{exact:true}).click();await page.getByText('CURRENT EVIDENCE',{exact:true}).waitFor();await page.waitForTimeout(1200);assert.equal(await page.getByText('OLD EVIDENCE',{exact:true}).count(),0);await page.unroute('**/api/v1/items/*/evidence');
 await page.getByRole('button',{name:'Back to results',exact:true}).click();
 await page.route('**/api/v1/items/*/evidence',route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'Temporary evidence failure'}}}));await page.locator(`[data-item-id="${seed.ids[0]}"]`).click();await page.getByText('Album details, evidence & files',{exact:true}).click();await page.getByRole('button',{name:'Retry evidence'}).waitFor();assert.equal(await page.getByText('No evidence recorded.',{exact:true}).count(),0);await page.unroute('**/api/v1/items/*/evidence');await page.getByRole('button',{name:'Retry evidence'}).click();await page.getByText('No evidence recorded.',{exact:true}).waitFor();
 if(process.env.BROWSE_SCREENSHOTS)await page.screenshot({path:path.join(process.env.BROWSE_SCREENSHOTS,'desktop-detail.png')});
 await page.goto(origin+'/?item='+randomUUID());await page.getByRole('button',{name:'Retry entry'}).waitFor();assert.equal(await page.locator('.detail').count(),0);
 await page.route('**/api/v1/items/'+seed.ids[0],route=>route.fulfill({status:403,json:{error:{code:'forbidden',message:'Access revoked'}}}));await page.goto(origin+'/?collection='+seed.music+'&item='+seed.ids[0]);await page.getByText('This entry is unavailable or you no longer have access.',{exact:false}).waitFor();assert.equal(await page.locator('.detail').count(),0);await page.unroute('**/api/v1/items/'+seed.ids[0]);await page.getByRole('button',{name:'Retry entry'}).click();await page.getByLabel('Album 000 details',{exact:true}).waitFor();
 // A directly opened detail has no earlier results entry to traverse back to.
 const directWork=await openWork();await directWork.getByRole('button',{name:'Close work history'}).click();await page.getByRole('button',{name:'Back to results',exact:true}).click();await page.waitForFunction(()=>!new URL(location.href).searchParams.has('item')&&!new URL(location.href).searchParams.has('work'));assert.equal(new URL(page.url()).searchParams.get('collection'),seed.music);assert.equal(await work.count(),0);
 await page.goto(origin+'/?collection='+seed.music);await page.locator('.collection-row').first().waitFor();
 await page.route('**/api/v1/library?*',route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'Temporary list failure'}}}));await page.getByLabel('Show',{exact:true}).selectOption('files_attached');await page.getByRole('button',{name:'Retry loading entries'}).waitFor();assert.equal(await page.getByRole('heading',{name:'No matching entries'}).count(),0);await page.unroute('**/api/v1/library?*');await page.getByRole('button',{name:'Retry loading entries'}).click();await page.getByRole('heading',{name:'No matching entries'}).waitFor();
 // Real unfinished uploads take precedence over playback attention. The list
 // must refresh after cancellation/completion without a manual reload.
 const beginImport=()=>page.evaluate(async ({collection,item})=>{
  async function request(url,body){const response=await fetch('/api/v1'+url,{method:body?'POST':'GET',headers:{'Content-Type':'application/json','Idempotency-Key':crypto.randomUUID()},body:body?JSON.stringify(body):undefined});if(!response.ok)throw Error(await response.text());return response.json();}
  const current=await request('/items/'+item);
  const plan=await request('/ingests/plan',{collection_id:collection,packages:[{source:{kind:'browser',label:null,package_name:'Library progress'},files:[{name:'original.bin',bytes:4,sha256:null}],anchor_file_index:null}]});
  const accepted=await request('/ingests',{plan,analysis:'preserve_only',enrichment:'none',prepare_music:false,destination:{item_id:item,expected_revision:current.revision}});
  return accepted.imports[0].import_id;
 },{collection:seed.music,item:seed.ids[0]});
 const progressImport=await beginImport();
 const concurrentImport=await beginImport();
 await page.goto(origin+'/?collection='+seed.music);
 const progressRow=page.locator(`[data-item-id="${seed.ids[0]}"]`);
 await expect(progressRow.getByText('Import in progress',{exact:true})).toBeVisible();
 await expect(progressRow.locator('.album-playback-status')).toHaveCount(0);
 await expect(progressRow).toHaveAccessibleName(/Import in progress/);
 if(process.env.BROWSE_SCREENSHOTS)await page.screenshot({path:path.join(process.env.BROWSE_SCREENSHOTS,'desktop-import-progress.png')});
 await page.setViewportSize({width:390,height:844});
 await expect(progressRow.getByText('Import in progress',{exact:true})).toBeVisible();
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);
 if(process.env.BROWSE_SCREENSHOTS)await page.screenshot({path:path.join(process.env.BROWSE_SCREENSHOTS,'mobile-import-progress.png')});
 const cancelImport=id=>page.evaluate(async id=>{
  const detail=await(await fetch('/api/v1/ingests/'+id)).json();
  const response=await fetch(`/api/v1/ingests/${id}/cancel`,{method:'POST',headers:{'Content-Type':'application/json','Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({expected_revision:detail.import.revision})});if(!response.ok)throw Error(await response.text());
 },id);
 await cancelImport(progressImport);
 assert.equal(await page.evaluate(async id=>(await(await fetch('/api/v1/items/'+id)).json()).import_in_progress,seed.ids[0]),true,'Another unfinished import must retain progress');
 await expect(progressRow.getByText('Import in progress',{exact:true})).toBeVisible();
 await cancelImport(concurrentImport);
 await expect(progressRow.getByText('Album playback needs attention',{exact:true})).toBeVisible();
 await expect(progressRow.getByText('Import in progress',{exact:true})).toHaveCount(0);
 const completedImport=await beginImport();
 await page.reload();await expect(progressRow.getByText('Import in progress',{exact:true})).toBeVisible();
 await page.evaluate(async id=>{const response=await fetch(`/api/v1/imports/${id}/files/0?offset=0`,{method:'PUT',headers:{'Content-Type':'application/octet-stream'},body:new Uint8Array([1,2,3,4])});if(!response.ok)throw Error(await response.text());},completedImport);
 await expect(progressRow.getByText('Album playback needs attention',{exact:true})).toBeVisible({timeout:30000});
 await expect(progressRow.getByText('Import in progress',{exact:true})).toHaveCount(0);
 assert.deepEqual(errors,[]);console.log(`FX-01 browser journey passed (${await browser.version()}): pagination/history/edit, nested work return and intentional browser Back/Forward, sorting/filtering, responsive keyboard navigation, delayed evidence, read recovery and live import progress.`);
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot({timeout:2000}).catch(()=> '[browser diagnostic unavailable]'));await page.screenshot({path:'/tmp/fx01-browser-failure.png',fullPage:true,timeout:2000}).catch(()=>{});}throw error;}finally{
 if(browser)await browser.close();await stopProcess(server);await rm(scratch,{recursive:true,force:true});
}
