// Offline real-server journey for the October 9 game import/catalog TODOs.
import {chromium,expect} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes,createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-game-catalog-'));
const config=path.join(scratch,'collector.toml'),password=randomBytes(24).toString('hex');
const origin='http://localhost:19972',binary=path.join(root,'target/debug/collector-server');
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19971"\nprivate_bind = "127.0.0.1:19972"\nlan_bind = "127.0.0.1:19973"\nallowed_origins = ["${origin}"]\n[exposure]\npublic_curation = false\npublic_administration = false\n`);
const setup=spawnSync(binary,['--config',config,'setup'],{input:password+'\n',encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let server,browser,page,logs='';
const cue=Buffer.from('FILE "one.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\nFILE "two.bin" BINARY\n TRACK 02 AUDIO\n INDEX 01 00:00:00\n');
const one=Buffer.alloc(2352*5),two=Buffer.alloc(2352*5,0x33);
const dat=(system,family,version,title,rows)=>Buffer.from(`<datafile><header><name>${system}</name><author>${family}</author><version>${version}</version></header><game name="${title}">${rows.map(([name,bytes])=>`<rom name="${name}" size="${bytes.length}" sha1="${createHash('sha1').update(bytes).digest('hex')}"/>`).join('')}</game></datafile>`);
const playstation=version=>dat('PSX','Redump',version,'Final Fantasy Tactics (USA)',[['Final Fantasy Tactics (USA) (Track 1).bin',one],['Final Fantasy Tactics (USA) (Track 2).bin',two]]);
const gb=dat('Game Boy','No-Intro','20261010','GB fixture',[['game.gb',Buffer.from('abc')]]);
try{
 server=spawn(binary,['--config',config,'serve']);server.stdout.on('data',v=>logs+=v);server.stderr.on('data',v=>logs+=v);
 let ready=false;for(let i=0;i<200;i++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 page=await browser.newPage({viewport:{width:1360,height:980}});page.setDefaultTimeout(15000);page.on('dialog',dialog=>dialog.accept());const errors=[],external=[];page.on('pageerror',e=>errors.push(e.message));page.on('response',async r=>{if(r.status()===422)console.error('Rejected request:',await r.text());});
 await page.route('**/*',route=>{const url=new URL(route.request().url());if(!['localhost','127.0.0.1'].includes(url.hostname)){external.push(url.toString());return route.abort();}return route.continue();});
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('heading',{name:'Library',exact:true}).waitFor();
 const collection=await page.evaluate(async()=>{const r=await fetch('/api/v1/admin/collections',{method:'POST',headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify({name:'Game fixtures',module:'org.junk.games'})});if(!r.ok)throw Error(await r.text());return r.json();});
 async function read(url){return page.evaluate(async url=>{const r=await fetch('/api/v1'+url);if(!r.ok)throw Error(await r.text());return r.json();},url);}
 async function importGame(){
  const previous=(await read(`/items?collection_id=${collection.id}&limit=50`)).items.filter(i=>i.captures?.length).length;
  await page.goto(`${origin}/?collection=${collection.id}`);await page.getByRole('button',{name:'Add / import',exact:true}).click();const editor=page.getByRole('dialog',{name:'Add or import',exact:true});
  await editor.locator('summary').filter({hasText:'Choose individual files instead'}).click();
  await editor.getByLabel('Files',{exact:true}).setInputFiles([{name:'disc.cue',mimeType:'text/plain',buffer:cue},{name:'one.bin',mimeType:'application/octet-stream',buffer:one},{name:'two.bin',mimeType:'application/octet-stream',buffer:two}]);
  await editor.getByRole('button',{name:'Preview import',exact:true}).click();await editor.getByText('1 dump packages found.',{exact:true}).waitFor();assert.equal(await editor.getByLabel('Prepare FLAC tracks after import').count(),0);
  await editor.getByRole('button',{name:'Save 1 package',exact:true}).click();
  await expect.poll(async()=>{const items=(await read(`/items?collection_id=${collection.id}&limit=50`)).items;return items.filter(i=>i.captures?.length).length;},{timeout:45000}).toBe(previous+1);
  await page.getByRole('button',{name:'Close curator',exact:true}).click();
  return (await read(`/items?collection_id=${collection.id}&limit=50`)).items.filter(i=>i.captures?.length);
 }
 const items=await importGame();assert.equal(items.length,1);const item=items[0];
 assert.equal(item.title,'disc');assert.equal(item.effective_metadata.title.source.source,'provisional');
 await page.getByRole('button',{name:'Administration',exact:true}).click();const admin=page.getByRole('dialog',{name:'Administration',exact:true});await admin.getByRole('button',{name:'Game catalogs',exact:true}).click();await admin.getByRole('button',{name:/Game fixtures/}).click();
 async function upload(files){await admin.getByLabel('DAT or ZIP files',{exact:true}).setInputFiles(files);await admin.getByRole('button',{name:'Detect and review catalogs',exact:true}).click();}
 await upload({name:'broken.zip',mimeType:'application/zip',buffer:Buffer.from('PKbad')});await expect(admin.getByRole('alert')).toContainText('Invalid ZIP');
 await upload({name:'broken.dat',mimeType:'application/octet-stream',buffer:Buffer.from('<datafile><game')});await expect(admin.getByRole('region',{name:'Detected catalogs'})).toContainText('Malformed');await expect(admin.getByRole('button',{name:'Import recognized catalogs'})).toBeDisabled();
 const zipPath=path.join(scratch,'catalogs.zip'),datPath=path.join(scratch,'playstation.dat');await writeFile(datPath,playstation('20261010'));
 const zipped=spawnSync('python3',['-c','import sys,zipfile; z=zipfile.ZipFile(sys.argv[2],"w",zipfile.ZIP_DEFLATED); z.write(sys.argv[1],"misleading.txt"); z.close()',datPath,zipPath]);assert.equal(zipped.status,0);
 await upload([{name:'catalogs.zip',mimeType:'application/zip',buffer:await readFile(zipPath)},{name:'gameboy.dat',mimeType:'application/octet-stream',buffer:gb}]);const detected=admin.getByRole('region',{name:'Detected catalogs'});await expect(detected).toContainText('Sony - PlayStation');await expect(detected).toContainText('Nintendo - Game Boy');
 assert.equal(await admin.getByLabel('Catalog platform').count(),0);await page.screenshot({path:'/tmp/collector-game-dat-review.png',fullPage:true});
 await admin.getByRole('button',{name:'Import recognized catalogs',exact:true}).click();await expect(admin.getByRole('button',{name:'Activate',exact:true})).toHaveCount(2,{timeout:45000});
 await admin.locator('.catalog-snapshot').filter({hasText:'Sony - PlayStation'}).filter({has:page.getByRole('button',{name:'Activate',exact:true})}).getByRole('button',{name:'Activate',exact:true}).click();
 await expect.poll(async()=>(await read(`/items/${item.id}`)).title,{timeout:45000}).toBe('Final Fantasy Tactics (USA)');
 await admin.locator('.catalog-snapshot').filter({hasText:'Nintendo - Game Boy'}).filter({has:page.getByRole('button',{name:'Activate',exact:true})}).getByRole('button',{name:'Activate',exact:true}).click();await expect(admin.getByRole('button',{name:'Active',exact:true})).toHaveCount(2);
 await expect.poll(async()=>{const rows=(await read(`/admin/collections/${collection.id}/catalog-snapshots?limit=50`)).items;return rows.filter(r=>r.active).length;}).toBe(2);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.screenshot({path:'/tmp/collector-game-dat-mobile.png',fullPage:true});await page.setViewportSize({width:1360,height:980});
 await admin.getByRole('button',{name:'Close administration',exact:true}).click();
 await page.goto(`${origin}/library-care/metadata?collection=${collection.id}&item=${item.id}&attention=all`);const review=page.getByRole('region',{name:'Metadata review',exact:true});await review.getByRole('heading',{name:'Current and detected fields'}).waitFor();
 await expect(review.getByLabel('Displayed title',{exact:true})).toHaveValue('Final Fantasy Tactics (USA)');await expect(review.getByLabel('Title mode',{exact:true})).toHaveValue('automatic');await expect(review.getByRole('heading',{name:'Game matches',exact:true})).toBeVisible();assert.doesNotMatch(await review.innerText(),/album/i);
 await review.getByLabel('Title mode',{exact:true}).selectOption('manual');await review.getByLabel('Displayed title',{exact:true}).fill('My manual edition');await review.getByRole('button',{name:'Save title',exact:true}).click();await expect.poll(async()=>(await read(`/items/${item.id}`)).title).toBe('My manual edition');
 await review.getByRole('button',{name:'Identify game',exact:true}).click();await expect(review.getByRole('button',{name:'Identify game',exact:true})).toBeEnabled({timeout:45000});assert.equal((await read(`/items/${item.id}`)).title,'My manual edition');
 await review.getByRole('button',{name:'Use detected title',exact:true}).click();await expect.poll(async()=>(await read(`/items/${item.id}`)).title).toBe('Final Fantasy Tactics (USA)');
 await review.getByLabel('System mode').selectOption('manual');await review.getByLabel('Selected system').selectOption('Sega - Saturn');await review.getByRole('button',{name:'Save system',exact:true}).click();await expect.poll(async()=>(await read(`/items/${item.id}`)).effective_metadata.domain_fields.platform.value).toBe('Sega - Saturn');await review.getByRole('button',{name:'Use detected system',exact:true}).click();await expect.poll(async()=>(await read(`/items/${item.id}`)).effective_metadata.domain_fields.platform.value).toBe('Sony - PlayStation');await expect.poll(async()=>(await read(`/items/${item.id}`)).effective_metadata.resolution).toBe('automatic');
 await page.screenshot({path:'/tmp/collector-game-existing-match.png',fullPage:true});
 await page.goto(`${origin}/?collection=${collection.id}&search=Final%20Fantasy`);await expect(page.locator('.collection-row').filter({hasText:'Final Fantasy Tactics (USA)'}).first()).toBeVisible();
 const future=await importGame();await expect.poll(async()=>{const rows=(await read(`/items?collection_id=${collection.id}&limit=50`)).items;return rows.filter(i=>i.captures?.length&&i.title==='Final Fantasy Tactics (USA)').length;},{timeout:45000}).toBe(2);
 assert.equal(future.length,2);assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
 console.log('Automatic game journey passed: provisional disc, mixed DAT/ZIP content detection, two active systems, activation scan, immediate title/system/search refresh, deliberate overrides and use-detected controls, future automatic imports, malformed upload rejection; offline.');
}catch(error){if(page){console.error((await page.locator('body').innerText()).slice(-14000));await page.screenshot({path:'/tmp/collector-game-failure.png',fullPage:true}).catch(()=>{});}throw error;}finally{await browser?.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(r=>server.once('exit',r)),new Promise(r=>setTimeout(()=>{server.kill('SIGKILL');r();},5000))]);}await rm(scratch,{recursive:true,force:true});}
