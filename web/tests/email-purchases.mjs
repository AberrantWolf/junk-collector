// Real Collector UI/storage acceptance against synthetic private receipt records.
import {chromium,expect} from './support/browser.mjs';
import {spawn,spawnSync,serverBinary,stopProcess,waitForServer} from './support/runtime.mjs';
import {mkdtemp,writeFile,rm,mkdir} from 'node:fs/promises';
import {createServer} from 'node:net';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
let scratch,server,browser,page,logs='';
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const password='Disposable-email-fixture-password';
try{
 scratch=await mkdtemp(path.join(tmpdir(),'collector-email-'));const publicPort=await port(),privatePort=await port(),lanPort=await port(),origin=`http://localhost:${privatePort}`;
 const config=path.join(scratch,'collector.toml');await writeFile(config,`archive="${scratch}/archive"\nstate="${scratch}/state"\npublic_bind="127.0.0.1:${publicPort}"\nprivate_bind="127.0.0.1:${privatePort}"\nlan_bind="127.0.0.1:${lanPort}"\nlocal_playback_enabled=false\nmulti_user=true\nallowed_origins=["${origin}"]\n`);
 const setup=spawnSync(serverBinary,['--config',config,'setup','--username','owner'],{input:password,encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
 async function start(){server=spawn(serverBinary,['--config',config,'serve'],{stdio:['ignore','pipe','pipe']});server.stderr.on('data',v=>logs+=v);await waitForServer(origin+'/healthz',server,{logs:()=>logs});}
 await start();browser=await chromium.launch();page=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByLabel('Password',{exact:true}).waitFor({state:'detached'});
 const ids=await page.evaluate(async()=>{const call=async(url,method='GET',body)=>{const r=await fetch('/api/v1'+url,{method,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},...(body?{body:JSON.stringify(body)}:{})});if(!r.ok)throw Error(await r.text());return r.json();};const actor=await call('/me');const collection=await call('/admin/collections','POST',{name:'Receipt games',module:'org.junk.games'});return {owner:actor.user_id,collection:collection.id};});
 await stopProcess(server);server=undefined;
 const seed=spawnSync('python3',['-c',`
import sqlite3,json,sys
p,owner,collection=sys.argv[1:]
db=sqlite3.connect(p)
for key,title,status in [('a','ゼルダ Collector Switch 2','preordered'),('b','ゼルダ Collector Switch 2','cancelled'),('c','Unwanted accessory','ordered')]:
 source=key*64
 purchase=dict(id=source,retailer='joshin',order_reference='J12345',title=title,platform='Nintendo Switch 2',edition='Collector',format='physical',status=status,observed_at='2026-10-'+('31' if key=='b' else '30')+'T10:00:00Z',intentional_duplicate=False,source_keys=[source])
 record=dict(id=source,connection_id='fixture',owner=owner,collection_id=collection,purchase=purchase,reason='Synthetic receipt for local UI acceptance.',dismissed=False,matches=[],accepted_item=None)
 db.execute('INSERT INTO purchase_suggestions(id,owner,collection_id,record) VALUES(?,?,?,?)',(source,owner,collection,json.dumps(record)))
db.commit()
`,path.join(scratch,'state/private.sqlite'),ids.owner,ids.collection],{encoding:'utf8'});assert.equal(seed.status,0,seed.stderr);
 await start();await page.goto(`${origin}/purchase-inbox?collection=${ids.collection}`);await page.getByRole('region',{name:'Purchase review inbox'}).waitFor();
 const first=page.locator('article').filter({hasText:'preordered'});await first.getByRole('button',{name:'Review suggestion'}).click();await first.getByLabel('Receipt item title').fill('The Legend of Zelda');await first.getByRole('checkbox',{name:'This duplicate purchase is intentional'}).check();await first.getByRole('button',{name:'Accept collection update'}).click();await expect(first).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('accepted');await expect(page.locator('article')).toHaveCount(1);await page.getByLabel('Review status',{exact:true}).selectOption('pending');
 const cancel=page.locator('article').filter({hasText:'cancelled'});await cancel.getByRole('button',{name:'Review suggestion'}).click();await cancel.getByLabel('Find an existing entry').fill('The Legend of Zelda');await cancel.getByRole('button',{name:'The Legend of Zelda',exact:true}).click();await cancel.getByLabel('Order action').selectOption('a'.repeat(64));await expect(cancel.getByRole('checkbox',{name:'This duplicate purchase is intentional'})).toBeChecked();await cancel.getByRole('button',{name:'Accept collection update'}).click();await expect(cancel).toHaveCount(0);
 const unwanted=page.locator('article').filter({hasText:'Unwanted accessory'});await unwanted.getByRole('button',{name:'Review suggestion'}).click();await unwanted.getByRole('button',{name:'Dismiss suggestion'}).click();await expect(unwanted).toHaveCount(0);
 await page.getByLabel('Review status',{exact:true}).selectOption('all');await expect(page.locator('article')).toHaveCount(3);await page.reload();await page.getByLabel('Review status',{exact:true}).selectOption('all');await expect(page.locator('article')).toHaveCount(3);
 const orders=await page.evaluate(async collection=>{const r=await fetch('/api/v1/library?collection_id='+collection);const items=(await r.json()).items;const detail=await (await fetch('/api/v1/items/'+items[0].id)).json();return {items:items.length,title:detail.title,specimens:detail.specimens.length,captures:detail.captures.length,orders:detail.purchase_orders};},ids.collection);
 assert.equal(orders.items,1);assert.equal(orders.title,'The Legend of Zelda');assert.equal(orders.specimens,0);assert.equal(orders.captures,0);assert.equal(orders.orders.length,1);assert.equal(orders.orders[0].status,'cancelled');assert.equal(orders.orders[0].intentional_duplicate,true);
 const screenshot=process.env.COLLECTOR_SCREENSHOTS;if(screenshot){await mkdir(screenshot,{recursive:true});await page.screenshot({path:path.join(screenshot,'purchase-inbox-desktop.png'),fullPage:true});}
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'mobile purchase inbox overflow');if(screenshot)await page.screenshot({path:path.join(screenshot,'purchase-inbox-mobile.png'),fullPage:true});
 assert.equal(await page.evaluate(async()=>{const r=await fetch('/purchase-inbox.js');return r.status;}),200);
 // The public listener does not compose the inbox page, asset or APIs.
 for(const route of ['/purchase-inbox','/purchase-inbox.js','/api/v1/mail/profiles','/api/v1/collections/'+ids.collection+'/purchase-inbox'])assert.equal((await fetch(`http://127.0.0.1:${publicPort}${route}`)).status,404,route);
 assert.deepEqual(errors,[]);console.log('Email purchase UI passed: accept, link/cancel, intentional duplicate, dismiss, restart, mobile and public listener denial.');
}catch(error){console.error(logs);if(page)console.error(await page.locator('body').innerText({timeout:2000}).catch(()=>''));throw error;}
finally{try{await browser?.close();}finally{await stopProcess(server);if(scratch)await rm(scratch,{recursive:true,force:true});}}
