import {clickAction} from './actions.mjs';
// FX-01 real-server navigation regression. All archive/state roots are temporary.
import {chromium,expect} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-browse-'));
const config=path.join(scratch,'collector.toml');
const binary=path.join(root,'target/debug/collector-server');
const password=randomBytes(24).toString('hex');
const origin='http://localhost:19942';
let server,browser,page,logs='';
try{
 await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19941"\nprivate_bind = "127.0.0.1:19942"\nlan_bind = "127.0.0.1:19943"\nlocal_playback_bind = "127.0.0.1:0"\nallowed_origins = []\n`);
 const init=spawnSync(binary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(init.status,0,init.stderr);
 server=spawn(binary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',v=>logs+=v);server.stdout.on('data',v=>logs+=v);
 let ready=false;for(let n=0;n<100;n++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
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
 await page.reload();await page.locator('.collection-row').first().waitFor();await page.waitForFunction(()=>document.querySelectorAll('.collection-row').length===50);
 assert.equal(await page.locator('.collection-row').count(),50);assert.match(await page.locator('.collection-row').first().innerText(),/Album 000/);
 const nextPageRequests=[];page.on('request',request=>{if(new URL(request.url()).pathname==='/api/v1/items')nextPageRequests.push(request.url());});
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
 await page.route('**/api/v1/items?*',async route=>{
  const response=await route.fetch();const page=await response.json();assert.ok(page.next_cursor);
  await route.fulfill({response,json:{...page,items:[]}});
 },{times:1});
 await page.getByLabel('Show',{exact:true}).selectOption('album_playback_needs_attention');
 await page.waitForFunction(()=>new URL(location.href).searchParams.get('filter')==='album_playback_needs_attention'&&document.querySelectorAll('.collection-row').length===12);
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
 await page.route('**/api/v1/items?*',route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'Temporary list failure'}}}));await page.getByLabel('Show',{exact:true}).selectOption('files_attached');await page.getByRole('button',{name:'Retry loading entries'}).waitFor();assert.equal(await page.getByRole('heading',{name:'No matching entries'}).count(),0);await page.unroute('**/api/v1/items?*');await page.getByRole('button',{name:'Retry loading entries'}).click();await page.getByRole('heading',{name:'No matching entries'}).waitFor();
 assert.deepEqual(errors,[]);console.log(`FX-01 browser journey passed (${await browser.version()}): pagination/history/edit, nested work return and intentional browser Back/Forward, sorting/filtering, responsive keyboard navigation, delayed evidence and read recovery.`);
}catch(error){if(page){console.error(await page.locator('body').ariaSnapshot());await page.screenshot({path:'/tmp/fx01-browser-failure.png',fullPage:true});}throw error;}finally{
 if(browser)await browser.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(r=>server.once('exit',r)),new Promise(r=>setTimeout(()=>{server.kill('SIGKILL');r();},5000))]);}await rm(scratch,{recursive:true,force:true});
}
