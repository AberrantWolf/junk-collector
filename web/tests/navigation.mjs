// Real-server acceptance for destination navigation and collection scope.
import {chromium,expect} from '@playwright/test';
import {spawn,spawnSync} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'../..');
const scratch=await mkdtemp(path.join(tmpdir(),'collector-navigation-'));
const config=path.join(scratch,'collector.toml');const password=randomBytes(24).toString('hex');
const origin='http://localhost:19962';const binary=path.join(root,'target/debug/collector-server');
await writeFile(config,`archive = "${scratch}/archive"\nstate = "${scratch}/state"\npublic_bind = "127.0.0.1:19961"\nprivate_bind = "127.0.0.1:19962"\nlan_bind = "127.0.0.1:19963"\nallowed_origins = ["${origin}", "http://localhost:19961"]\n[exposure]\npublic_curation = false\npublic_administration = false\n`);
const setup=spawnSync(binary,['--config',config,'setup'],{input:password+'\n',encoding:'utf8'});assert.equal(setup.status,0,setup.stderr);
let server,browser;let logs='';
try{
 server=spawn(binary,['--config',config,'serve']);server.stdout.on('data',value=>logs+=value);server.stderr.on('data',value=>logs+=value);
 let ready=false;for(let i=0;i<200;i++){try{if((await fetch(origin+'/healthz')).ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,100));}assert.ok(ready,logs);
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const page=await browser.newPage({viewport:{width:1360,height:980}});page.setDefaultTimeout(15000);const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(origin);await page.getByLabel('Password',{exact:true}).fill(password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('heading',{name:'Library',exact:true}).waitFor();
 const seed=await page.evaluate(async()=>{
  async function request(url,data,method='POST'){const response=await fetch('/api/v1'+url,{method,headers:{'content-type':'application/json','idempotency-key':crypto.randomUUID()},body:JSON.stringify(data)});if(!response.ok)throw new Error(await response.text());return response.json();}
  const collections=[];for(const name of ['CD collection','Cartridge shelf','Concert recordings','Soundtracks'])collections.push(await request('/admin/collections',{name,module:'org.junk.music'}));
  for(const [title,artist,subtitle] of [['Blue Train','John Coltrane','1957 · CD edition'],['Kind of Blue','Miles Davis','1959 · CD edition'],['Head Hunters','Herbie Hancock','1973 · CD edition'],['Time Out','The Dave Brubeck Quartet','1959 · CD edition'],['Maiden Voyage','Herbie Hancock','1965 · CD edition']])await request('/items/'+crypto.randomUUID(),{expected_revision:0,edit:{collection_id:collections[0].id,title,subtitle,identity_link:{kind:'none'},domain_edit:{module:'org.junk.music',fields:{artist}},identity:'unidentified',translations:[],identifiers:[],wishlist:false}},'PUT');
  return collections;
 });
 await page.reload();await expect(page.locator('.collection-row')).toHaveCount(5);const nav=page.getByRole('navigation',{name:'Collection tasks'});await nav.getByRole('button',{name:'Activity',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Work & history',exact:true}).count(),0);assert.equal(await page.getByRole('button',{name:'Manage collections',exact:true}).count(),0);await expect(nav.locator('[aria-current="page"]')).toHaveCount(1);
 assert.equal(await page.locator('.sidebar').getByRole('searchbox').count(),0);
 await page.screenshot({path:'/tmp/collector-navigation-desktop.png',fullPage:true});
 const scope=page.locator('.collection-scope');const summary=scope.locator('summary');await summary.focus();await page.keyboard.press('Enter');await page.getByRole('button',{name:'CD collection',exact:true}).click();await expect(summary).toContainText('CD collection');await expect(summary).toBeFocused();assert.equal(await scope.getAttribute('open'),null);
 await nav.getByRole('button',{name:'Lists',exact:true}).click();await expect(page.getByRole('heading',{name:'Lists',exact:true})).toBeVisible();assert.equal(new URL(page.url()).searchParams.get('collection'),seed[0].id);await expect(nav.getByRole('button',{name:'Lists',exact:true})).toHaveAttribute('aria-current','page');
 await page.goBack();await expect(page.getByRole('heading',{name:'Library',exact:true})).toBeVisible();await expect(summary).toContainText('CD collection');
 // Force smaller real pages to exercise continuation and changing-query fences.
 await page.route('**/api/v1/collection-search?*',route=>{const url=new URL(route.request().url());url.searchParams.set('limit','2');return route.continue({url:url.toString()});});
 await summary.click();await page.getByRole('searchbox',{name:'Find a collection'}).fill('C');await page.getByRole('button',{name:'More collections',exact:true}).waitFor();await page.getByRole('button',{name:'More collections',exact:true}).click();await page.getByRole('button',{name:'Concert recordings',exact:true}).waitFor();await page.getByRole('searchbox',{name:'Find a collection'}).fill('Soundtracks');await page.getByRole('button',{name:'Soundtracks',exact:true}).waitFor();assert.equal(await page.getByRole('button',{name:'More collections',exact:true}).count(),0);
 await page.keyboard.press('Escape');await expect(summary).toBeFocused();assert.equal(await scope.getAttribute('open'),null);await page.unroute('**/api/v1/collection-search?*');
 await page.setViewportSize({width:390,height:844});const toggle=page.locator('.menu-toggle');await expect(toggle).toBeVisible();await expect(nav).toBeHidden();assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.screenshot({path:'/tmp/collector-navigation-mobile.png',fullPage:false});await toggle.focus();await page.keyboard.press('Enter');await expect(nav).toBeVisible();await expect(toggle).toHaveAttribute('aria-expanded','true');assert.equal(await nav.evaluate(node=>node.scrollWidth>node.clientWidth),false);const destinations=await nav.boundingBox();const manage=await page.locator('.management-navigation').boundingBox();assert.ok(destinations.y+destinations.height<=manage.y+1,'mobile navigation groups overlap');await page.screenshot({path:'/tmp/collector-navigation-mobile-menu.png',fullPage:false});await page.getByRole('button',{name:'Sign out',exact:true}).scrollIntoViewIfNeeded();await expect(page.getByRole('button',{name:'Sign out',exact:true})).toBeInViewport();
 await nav.getByRole('button',{name:'Metadata',exact:true}).click();await expect(page.getByRole('heading',{name:'Metadata',exact:true})).toBeVisible();await expect(nav).toBeHidden();await expect(page.getByRole('button',{name:'Menu',exact:true})).toBeFocused();await expect(summary).toContainText('CD collection');
 await page.getByRole('button',{name:'Menu',exact:true}).click();await page.keyboard.press('Escape');await expect(nav).toBeHidden();await expect(page.getByRole('button',{name:'Menu',exact:true})).toBeFocused();
 await page.getByRole('button',{name:'Menu',exact:true}).click();await page.getByRole('button',{name:'Account & security',exact:true}).click();await page.getByRole('dialog',{name:'Account & security'}).waitFor();await page.getByRole('button',{name:'Close account and security',exact:true}).click();await expect(page.getByRole('button',{name:'Menu',exact:true})).toBeFocused();
 await page.setViewportSize({width:1360,height:600});await page.getByRole('button',{name:'Sign out',exact:true}).scrollIntoViewIfNeeded();await expect(page.getByRole('button',{name:'Sign out',exact:true})).toBeInViewport();
 const client=await browser.newPage();await client.goto('http://localhost:19961');await client.getByLabel('Password',{exact:true}).fill(password);await client.getByRole('button',{name:'Sign in',exact:true}).click();await client.getByRole('heading',{name:'Library',exact:true}).waitFor();assert.equal(await client.getByRole('button',{name:'Administration',exact:true}).count(),0);assert.equal(await client.getByRole('button',{name:'Imports',exact:true}).count(),0);await expect(client.getByRole('button',{name:'Lists',exact:true})).toBeVisible();
 assert.deepEqual(errors,[]);console.log('Navigation acceptance passed: destination/scope separation, populated desktop/mobile screenshots, scope preservation, browser back, search/pagination, keyboard disclosures and focus, mobile/short viewport access, listener exposure.');
}finally{await browser?.close();if(server&&server.exitCode===null){server.kill('SIGINT');await Promise.race([new Promise(resolve=>server.once('exit',resolve)),new Promise(resolve=>setTimeout(()=>{server.kill('SIGKILL');resolve();},5000))]);}await rm(scratch,{recursive:true,force:true});}
