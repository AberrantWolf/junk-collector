// Real Collector UI/storage acceptance against synthetic private receipt records.
import {chromium,expect} from './support/browser.mjs';
import {spawn,spawnSync,serverBinary,stopProcess,waitForServer} from './support/runtime.mjs';
import {mkdtemp,writeFile,readFile,rename,rm,mkdir} from 'node:fs/promises';
import {createServer} from 'node:net';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash,X509Certificate} from 'node:crypto';
import {fileURLToPath} from 'node:url';
let scratch,server,bridge,browser,page,logs='',heartbeat;let stale=false,partial=false;
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const password='Disposable-email-fixture-password';
try{
 scratch=await mkdtemp(path.join(tmpdir(),'collector-email-'));const publicPort=await port(),privatePort=await port(),lanPort=await port(),origin=`http://localhost:${privatePort}`;
 const profiles=path.join(scratch,'profiles.json'),managed=path.join(scratch,'mail-profiles'),status=path.join(scratch,'bridge-status.json'),certificate=path.join(scratch,'cert.pem'),key=path.join(scratch,'key.pem'),mailCommands=path.join(scratch,'mail-commands.log'),mailControls=path.join(scratch,'mail-controls.json');
 await writeFile(mailControls,'{}');await writeFile(profiles,'[]',{mode:0o600});await mkdir(managed);
 const cert=spawnSync('openssl',['req','-x509','-newkey','rsa:2048','-nodes','-subj','/CN=localhost','-days','1','-keyout',key,'-out',certificate],{encoding:'utf8'});assert.equal(cert.status,0,cert.stderr);
 bridge=spawn('python3',['-u',fileURLToPath(new URL('./support/mail-bridge.py',import.meta.url)),certificate,key,mailCommands,mailControls],{stdio:['ignore','pipe','pipe']});
 const imapPort=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('IMAP fixture startup timed out')),10000);bridge.stdout.once('data',v=>{clearTimeout(timer);resolve(Number(v.toString().trim()));});bridge.once('exit',()=>{clearTimeout(timer);reject(Error('IMAP fixture exited before listening'));});});
 const fingerprint=createHash('sha256').update(new X509Certificate(await readFile(certificate)).raw).digest('hex');
 async function report(){const now=Math.floor(Date.now()/1000);await writeFile(status+'.pending',JSON.stringify({schema:1,imap_port:imapPort,observed_at:stale?now-60:now,event_at:partial?null:stale?now-60:now,phase:partial?'synced':'syncing',percent:partial?null:37.5,elapsed_seconds:12,remaining_seconds:partial?null:20,certificate_sha256:fingerprint}));await rename(status+'.pending',status);}
 await report();heartbeat=setInterval(()=>void report().catch(()=>{}),1000);
 const config=path.join(scratch,'collector.toml');await writeFile(config,`archive="${scratch}/archive"\nstate="${scratch}/state"\npublic_bind="127.0.0.1:${publicPort}"\nprivate_bind="127.0.0.1:${privatePort}"\nlan_bind="127.0.0.1:${lanPort}"\nlocal_playback_enabled=false\nmulti_user=true\nallowed_origins=["${origin}"]\n`);
 const setup=spawnSync(serverBinary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
 async function start(){server=spawn(serverBinary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe'],env:{...process.env,COLLECTOR_MAIL_PROFILES:profiles,COLLECTOR_MAIL_SETUP_DIRECTORY:managed,COLLECTOR_BRIDGE_STATUS:status}});server.stderr.on('data',v=>logs+=v);await waitForServer(origin+'/healthz',server,{logs:()=>logs});}
 await start();browser=await chromium.launch();page=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByLabel('Password',{exact:true}).waitFor({state:'detached'});
 const ids=await page.evaluate(async()=>{const call=async(url,method='GET',body)=>{const r=await fetch('/api/v1'+url,{method,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},...(body?{body:JSON.stringify(body)}:{})});if(!r.ok)throw Error(await r.text());return r.json();};const actor=await call('/me');const collection=await call('/admin/collections','POST',{name:'Receipt games',module:'org.junk.games'});return {owner:actor.user_id,collection:collection.id};});
 await page.goto(`${origin}/purchase-inbox?collection=${ids.collection}`);
 const submitted=[];page.on('request',r=>{if(r.url().endsWith('/api/v1/mail/setup')&&r.method()==='POST')submitted.push({key:r.headers()['idempotency-key'],body:r.postData()});});
 const connection=page.getByRole('region',{name:'Mail connection',exact:true});
 await connection.getByRole('progressbar',{name:'Bridge mailbox sync'}).waitFor();
 const setupShots=process.env.COLLECTOR_SCREENSHOTS;if(setupShots){await mkdir(setupShots,{recursive:true});await page.screenshot({path:path.join(setupShots,'mail-setup-desktop.png'),fullPage:true});}await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile mail setup overflow');if(setupShots)await page.screenshot({path:path.join(setupShots,'mail-setup-mobile.png'),fullPage:true});await page.setViewportSize({width:1440,height:1000});
 await connection.getByLabel('Bridge IMAP username',{exact:true}).fill('fixture-local');await connection.getByLabel('Bridge IMAP password',{exact:true}).fill('incorrect-private-password');
 await connection.getByRole('button',{name:'Connect Bridge',exact:true}).click();await connection.getByRole('alert').filter({hasText:'Could not connect'}).waitFor();
 assert.equal(await page.evaluate(async()=>{const r=await fetch('/api/v1/mail/profiles');return (await r.json()).length;}),0);
 await connection.getByLabel('Bridge IMAP password',{exact:true}).fill('fixture-imap-password');await connection.getByRole('button',{name:'Connect Bridge',exact:true}).click();await connection.getByRole('heading',{name:'My Proton Mail · Connected'}).waitFor();
 const configured=await page.evaluate(async()=>({profiles:await (await fetch('/api/v1/mail/profiles')).json(),connections:await (await fetch('/api/v1/mail/connections')).json()}));
 assert.equal(configured.profiles.length,1);assert.equal(configured.connections.length,0);assert.ok(!JSON.stringify(configured).includes('fixture-imap-password'));
 await page.reload();await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();
 await connection.getByLabel('Mail folders, one per line',{exact:true}).fill(Array(17).fill('INBOX').join('\n'));await connection.getByRole('button',{name:'Save folders',exact:true}).click();await connection.getByRole('alert').filter({hasText:'1–16'}).waitFor();
 await connection.getByLabel('Mail folders, one per line',{exact:true}).fill('INBOX');await connection.getByRole('button',{name:'Save folders',exact:true}).click();await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).waitFor();
 await expect(connection.getByRole('button',{name:'Enable receipt scanning',exact:true})).toBeVisible();
 await connection.getByRole('button',{name:'Enable receipt scanning',exact:true}).click();await connection.getByRole('button',{name:'Pause receipt scanning',exact:true}).waitFor();
 await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();
 await connection.getByLabel('Mail folders, one per line',{exact:true}).fill('INBOX\nFolders/Receipts');
 // Polling must preserve unsaved edits.
 await connection.getByRole('button',{name:'Refresh mail status',exact:true}).click();
 await expect(connection.getByLabel('Mail folders, one per line',{exact:true})).toHaveValue('INBOX\nFolders/Receipts');
 await connection.getByRole('button',{name:'Save folders',exact:true}).click();
 await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).waitFor();
 await expect(connection.getByRole('button',{name:'Pause receipt scanning',exact:true})).toBeVisible();
 await page.reload();await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();
 await expect(connection.getByLabel('Mail folders, one per line',{exact:true})).toHaveValue('INBOX\nFolders/Receipts');
 const folderShots=process.env.COLLECTOR_SCREENSHOTS;if(folderShots)await page.screenshot({path:path.join(folderShots,'mail-folders-desktop.png'),fullPage:true});
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile folder editor overflow');if(folderShots)await page.screenshot({path:path.join(folderShots,'mail-folders-mobile.png'),fullPage:true});await page.setViewportSize({width:1440,height:1000});
 await connection.getByLabel('Mail folders, one per line',{exact:true}).fill('discarded');await connection.getByRole('button',{name:'Cancel folder editing',exact:true}).click();
 await connection.getByRole('button',{name:'Pause receipt scanning',exact:true}).click();await connection.getByRole('button',{name:'Enable receipt scanning',exact:true}).waitFor();
 await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();
 await connection.getByLabel('Mail folders, one per line',{exact:true}).fill('Folders/Receipts');
 await connection.getByRole('button',{name:'Save folders',exact:true}).click();await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).waitFor();
 await expect(connection.getByRole('button',{name:'Enable receipt scanning',exact:true})).toBeVisible();
 // A concurrent settings change is visible and cannot be overwritten by an old draft.
 await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();
 assert.equal(await page.evaluate(async()=>{const c=(await (await fetch('/api/v1/mail/connections')).json())[0];return (await fetch('/api/v1/mail/connections/'+c.profile.id,{method:'PUT',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({collection_id:c.collection_id,enabled:false,expected_revision:c.revision,folders:['Folders/Other']})})).status;}),200);
 await connection.getByRole('button',{name:'Save folders',exact:true}).click();await connection.getByRole('alert').filter({hasText:'changed'}).waitFor();
 await expect(connection.getByLabel('Mail folders, one per line',{exact:true})).toHaveValue('Folders/Receipts');
 await connection.getByRole('button',{name:'Cancel folder editing',exact:true}).click();await connection.getByRole('button',{name:'Refresh mail status',exact:true}).click();
 await expect(connection.getByText('Scanned folders: Folders/Other',{exact:true})).toBeVisible();
 await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();await expect(connection.getByLabel('Mail folders, one per line',{exact:true})).toHaveValue('Folders/Other');
 await connection.getByLabel('Mail folders, one per line',{exact:true}).fill('Folders/Receipts');await connection.getByRole('button',{name:'Save folders',exact:true}).click();await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).waitFor();
 const folderConnection=await page.evaluate(async()=>(await (await fetch('/api/v1/mail/connections')).json())[0]);
 assert.deepEqual(folderConnection.folders,['Folders/Receipts']);assert.equal(folderConnection.enabled,false);

 const successRequest=submitted.at(-1);
 stale=true;await report();await connection.getByRole('button',{name:'Refresh mail status',exact:true}).click();await connection.getByRole('alert').filter({hasText:'unavailable or stale'}).waitFor();await expect(connection.getByRole('progressbar')).toHaveCount(0);
 assert.equal(await page.evaluate(async request=>(await fetch('/api/v1/mail/setup',{method:'POST',headers:{'content-type':'application/json','idempotency-key':request.key},body:request.body})).status,successRequest),200);
 assert.equal(await page.evaluate(async request=>(await fetch('/api/v1/mail/setup',{method:'POST',headers:{'content-type':'application/json','idempotency-key':request.key},body:JSON.stringify({...JSON.parse(request.body),username:'different-input'})})).status,successRequest),409);
 stale=false;await report();await connection.getByRole('button',{name:'Refresh mail status',exact:true}).click();await connection.getByRole('progressbar').waitFor();
 partial=true;await report();await connection.getByRole('button',{name:'Refresh mail status',exact:true}).click();await expect(connection.getByRole('progressbar')).toHaveCount(0);assert.equal(await page.evaluate(async()=>(await (await fetch('/api/v1/mail/setup')).json()).bridge),null);partial=false;await report();await connection.getByRole('button',{name:'Refresh mail status',exact:true}).click();await connection.getByRole('progressbar').waitFor();
 const settings=await page.evaluate(async()=>{const r=await fetch('/api/v1/admin/users',{method:'POST',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({username:'mail-viewer',password:'Disposable-mail-viewer-password',administrator:false})});if(!r.ok)throw Error(await r.text());return r.json();});
 const viewer=await browser.newContext();const viewerPage=await viewer.newPage();await viewerPage.goto(origin);await viewerPage.getByLabel('Username',{exact:true}).fill('mail-viewer');await viewerPage.getByLabel('Password',{exact:true}).fill('Disposable-mail-viewer-password');await viewerPage.getByRole('button',{name:'Sign in',exact:true}).click();await viewerPage.getByLabel('Password',{exact:true}).waitFor({state:'detached'});
 for(const method of ['GET','POST'])assert.equal(await viewerPage.evaluate(async method=>(await fetch('/api/v1/mail/setup',{method,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},...(method==='POST'?{body:JSON.stringify({username:'fixture-local',password:'fixture-imap-password',folders:['INBOX']})}:{})})).status,method),403);
 assert.equal(await viewerPage.evaluate(async c=>(await fetch('/api/v1/mail/profiles/'+c.profile.id+'/repair',{method:'POST',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({username:'fixture-local',password:'private',certificate_sha256:'a'.repeat(64),expected_revision:0})})).status,folderConnection),403);
 assert.equal(await viewerPage.evaluate(async c=>(await fetch('/api/v1/mail/connections/'+c.profile.id,{method:'PUT',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({collection_id:c.collection_id,enabled:false,expected_revision:c.revision,folders:['INBOX']})})).status,folderConnection),404);
 await viewer.close();void settings;
 await connection.getByRole('button',{name:'Enable receipt scanning',exact:true}).click();await connection.getByRole('button',{name:'Pause receipt scanning',exact:true}).waitFor();
 await stopProcess(server);server=undefined;
 const seed=spawnSync('python3',['-c',`
import sqlite3,json,sys
p,owner,collection=sys.argv[1:]
db=sqlite3.connect(p)
for key,title,status in [('a','ゼルダ Collector Switch 2','preordered'),('b','ゼルダ Collector Switch 2','cancelled'),('c','Unwanted accessory','ordered'),('d','Unparsed receipt — review original email','unknown')]:
 source=key*64
 purchase=dict(id=source,retailer='joshin',order_reference='J12345',title=title,platform='Nintendo Switch 2',edition='Collector',format='physical',status=status,observed_at='2026-10-'+('31' if key=='b' else '30')+'T10:00:00Z',intentional_duplicate=False,source_keys=[source])
 record=dict(id=source,connection_id='fixture',owner=owner,collection_id=collection,purchase=purchase,reason='Synthetic receipt for local UI acceptance.',dismissed=False,matches=[],accepted_item=None)
 if key=='d':record.update(issue='unsupported_content',source=dict(folder='Folders/Receipts',uid_validity=42,uid=9,message_id='<private-fixture-receipt>',subject='Synthetic unparsed purchase receipt',date='Fri, 30 Oct 2026 10:00:00 +0000'))
 db.execute('INSERT INTO purchase_suggestions(id,owner,collection_id,record) VALUES(?,?,?,?)',(source,owner,collection,json.dumps(record)))
db.commit()
`,path.join(scratch,'state/private.sqlite'),ids.owner,ids.collection],{encoding:'utf8'});assert.equal(seed.status,0,seed.stderr);
 await writeFile(mailControls,JSON.stringify({password:'fixture-rotated-password',uidnext:201}));
 await start();await page.goto(`${origin}/purchase-inbox?collection=${ids.collection}`);await page.getByRole('region',{name:'Purchase review inbox'}).waitFor();
 await connection.getByRole('alert').filter({hasText:'rejected the local credentials'}).waitFor();
 assert.equal(await page.evaluate(async()=>(await (await fetch('/api/v1/mail/connections')).json())[0].failure.kind),'authentication');
 const beforeRepair=await page.evaluate(async()=>(await (await fetch('/api/v1/mail/connections')).json())[0]);
 await connection.getByRole('button',{name:'Revalidate Bridge connection',exact:true}).click();
 await connection.getByLabel('Current Bridge IMAP username',{exact:true}).fill('fixture-local');await connection.getByLabel('Current Bridge IMAP password',{exact:true}).fill('wrong-rotated-password');
 await connection.getByRole('checkbox',{name:'I verified this certificate fingerprint in the private Bridge terminal.'}).check();
 await connection.getByRole('button',{name:'Save revalidated connection',exact:true}).click();await connection.getByRole('alert').filter({hasText:'Could not revalidate'}).waitFor();
 await connection.getByLabel('Current Bridge IMAP password',{exact:true}).fill('fixture-rotated-password');
 const repairsShots=process.env.COLLECTOR_SCREENSHOTS;if(repairsShots)await page.screenshot({path:path.join(repairsShots,'mail-revalidation-desktop.png'),fullPage:true});await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile revalidation overflow');if(repairsShots)await page.screenshot({path:path.join(repairsShots,'mail-revalidation-mobile.png'),fullPage:true});await page.setViewportSize({width:1440,height:1000});
 await connection.getByRole('button',{name:'Save revalidated connection',exact:true}).click();await connection.getByRole('button',{name:'Revalidate Bridge connection',exact:true}).waitFor();
 const afterRepair=await page.evaluate(async()=>(await (await fetch('/api/v1/mail/connections')).json())[0]);assert.equal(afterRepair.profile.id,beforeRepair.profile.id);assert.equal(afterRepair.cursor,beforeRepair.cursor);assert.equal(afterRepair.enabled,beforeRepair.enabled);await connection.getByText('Bridge configuration was updated after the last scan problem. Waiting for a new scan.',{exact:true}).waitFor();
 await stopProcess(server);server=undefined;await start();await page.reload();
 await connection.getByRole('region',{name:'Receipt history progress'}).getByText(/100 UID positions remaining/).waitFor();
 await connection.getByRole('button',{name:'Rescan configured folders',exact:true}).click();await connection.getByRole('button',{name:'Pause receipt scanning',exact:true}).waitFor();
 await expect.poll(async()=>await page.evaluate(async()=>(await (await fetch('/api/v1/mail/connections')).json())[0].cursor)).toBe('');
 await stopProcess(server);server=undefined;await start();await page.reload();
 await connection.getByRole('region',{name:'Receipt history progress'}).getByText(/100 UID positions remaining/).waitFor();
 const unparsed=page.locator('article').filter({hasText:'Unparsed receipt — review original email'});await unparsed.getByText('Find original email',{exact:true}).click();await expect(unparsed.getByText('Subject: Synthetic unparsed purchase receipt',{exact:true})).toBeVisible();await unparsed.getByRole('button',{name:'Review suggestion',exact:true}).click();await unparsed.getByRole('button',{name:'Dismiss suggestion',exact:true}).click();await expect(unparsed).toHaveCount(0);

 await connection.getByRole('button',{name:'Edit scanned folders',exact:true}).click();await expect(connection.getByLabel('Mail folders, one per line',{exact:true})).toHaveValue('Folders/Receipts');await connection.getByRole('button',{name:'Cancel folder editing',exact:true}).click();
 await expect.poll(async()=>await readFile(mailCommands,'utf8'),{timeout:20000}).toContain('EXAMINE "Folders/Receipts"');
 const first=page.locator('article').filter({hasText:'preordered'});await first.getByRole('button',{name:'Review suggestion'}).click();await first.getByLabel('Receipt item title').fill('The Legend of Zelda');await first.getByRole('checkbox',{name:'This duplicate purchase is intentional'}).check();await first.getByRole('button',{name:'Accept collection update'}).click();await expect(first).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('accepted');await expect(page.locator('article')).toHaveCount(1);await page.getByLabel('Review status',{exact:true}).selectOption('pending');
 const cancel=page.locator('article').filter({hasText:'cancelled'});await cancel.getByRole('button',{name:'Review suggestion'}).click();await cancel.getByLabel('Find an existing entry').fill('The Legend of Zelda');await cancel.getByRole('button',{name:'The Legend of Zelda',exact:true}).click();await cancel.getByLabel('Order action').selectOption('a'.repeat(64));await expect(cancel.getByRole('checkbox',{name:'This duplicate purchase is intentional'})).toBeChecked();await cancel.getByRole('button',{name:'Accept collection update'}).click();await expect(cancel).toHaveCount(0);
 const unwanted=page.locator('article').filter({hasText:'Unwanted accessory'});await unwanted.getByRole('button',{name:'Review suggestion'}).click();await unwanted.getByRole('button',{name:'Dismiss suggestion'}).click();await expect(unwanted).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('all');await expect(page.locator('article')).toHaveCount(4);await page.reload();await page.getByLabel('Review status',{exact:true}).selectOption('all');await expect(page.locator('article')).toHaveCount(4);
 const orders=await page.evaluate(async collection=>{const r=await fetch('/api/v1/library?collection_id='+collection);const items=(await r.json()).items;const detail=await (await fetch('/api/v1/items/'+items[0].id)).json();return {id:detail.id,items:items.length,title:detail.title,specimens:detail.specimens.length,captures:detail.captures.length,orders:detail.purchase_orders};},ids.collection);
 assert.equal(orders.items,1);assert.equal(orders.title,'The Legend of Zelda');assert.equal(orders.specimens,0);assert.equal(orders.captures,0);assert.equal(orders.orders.length,1);assert.equal(orders.orders[0].status,'cancelled');assert.equal(orders.orders[0].intentional_duplicate,true);
 // Restore an accidental dismissal, then explicitly reopen an accepted order for correction/reassignment.
 await page.getByLabel('Review status',{exact:true}).selectOption('dismissed');const dismissed=page.locator('article').filter({hasText:'Unwanted accessory'});await dismissed.getByRole('button',{name:'Restore suggestion',exact:true}).click();await expect(dismissed).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('pending');const restored=page.locator('article').filter({hasText:'Unwanted accessory'});await restored.getByRole('button',{name:'Review suggestion',exact:true}).click();await restored.getByRole('button',{name:'Dismiss suggestion',exact:true}).click();await expect(restored).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('accepted');await expect(page.locator('article')).toHaveCount(2);const accepted=page.locator('article').filter({hasText:'cancelled'});await accepted.getByRole('button',{name:'Reopen purchase review',exact:true}).click();await accepted.getByRole('button',{name:'Confirm reopen purchase',exact:true}).click();await expect(page.locator('article')).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('pending');const correction=page.locator('article').filter({hasText:'preordered'});await correction.getByRole('button',{name:'Review suggestion',exact:true}).click();await correction.getByLabel('Receipt item title').fill('Corrected Zelda');await correction.getByLabel('Platform',{exact:true}).fill('Corrected platform');await correction.getByRole('button',{name:'Accept collection update',exact:true}).click();await expect(correction).toHaveCount(0);
 const reassigned=await page.evaluate(async id=>{const r=await fetch('/api/v1/items/'+id);if(!r.ok)throw Error(await r.text());const old=await r.json();return old.purchase_orders??[];},orders.id);assert.deepEqual(reassigned,[]);
 await page.reload();await page.getByLabel('Review status',{exact:true}).selectOption('accepted');await expect(page.locator('article')).toHaveCount(1);
 const screenshot=process.env.COLLECTOR_SCREENSHOTS;if(screenshot){await mkdir(screenshot,{recursive:true});await page.screenshot({path:path.join(screenshot,'purchase-inbox-desktop.png'),fullPage:true});}
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile purchase inbox overflow');if(screenshot)await page.screenshot({path:path.join(screenshot,'purchase-inbox-mobile.png'),fullPage:true});
 assert.equal(await page.evaluate(async()=>{const r=await fetch('/purchase-inbox.js');return r.status;}),200);
 // The public listener does not compose the inbox page, asset or APIs.
 for(const route of ['/purchase-inbox','/purchase-inbox.js','/api/v1/mail/profiles','/api/v1/collections/'+ids.collection+'/purchase-inbox'])assert.equal((await fetch(`http://127.0.0.1:${publicPort}${route}`)).status,404,route);
 assert.deepEqual(errors,[]);console.log('Email purchase UI passed: accept, link/cancel, intentional duplicate, dismiss, restart, mobile and public listener denial.');
}catch(error){console.error(logs);if(page)console.error(await page.locator('body').innerText({timeout:2000}).catch(()=>''));throw error;}
finally{try{await browser?.close();}finally{clearInterval(heartbeat);const stopped=await Promise.allSettled([stopProcess(server),stopProcess(bridge)]);if(scratch)await rm(scratch,{recursive:true,force:true});for(const result of stopped)if(result.status==='rejected')throw result.reason;}}
