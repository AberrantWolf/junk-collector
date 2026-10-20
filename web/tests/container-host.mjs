// Browser handoff for check_container.py, including actual remote-host TLS/media.
import {chromium,expect} from '@playwright/test';
import {readFile,mkdtemp} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import https from 'node:https';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {clickAction} from './actions.mjs';
const fixture=JSON.parse(await readFile(process.argv[2],'utf8'));
const scratch=await mkdtemp(path.join(tmpdir(),'collector-container-browser-'));
// Validate the isolated host's actual certificate/name/server identity before login.
const identity=await new Promise((resolve,reject)=>{const req=https.get(fixture.origin+'/api/v1/identity',{ca:fixture.certificate_pem,...fixture.private_ip?{family:4,lookup:(_hostname,_options,callback)=>callback(null,fixture.private_ip,4)}:{}},response=>{let body='';response.on('data',part=>body+=part);response.on('end',()=>{try{assert.equal(response.statusCode,200);resolve(JSON.parse(body));}catch(error){reject(error);}});});req.setTimeout(10000,()=>req.destroy(Error('Private HTTPS identity timed out')));req.on('error',reject);});
assert.equal(identity.server_id,fixture.server_id);
let browser,page;
try{
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true,args:fixture.private_ip?[`--host-resolver-rules=MAP ${new URL(fixture.origin).hostname} ${fixture.private_ip}`]:[]});
 // Test-only self-signed browser acceptance; the PEM/name was independently validated above.
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:{width:1440,height:1000}});context.setDefaultTimeout(30000);
 page=await context.newPage();const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(fixture.origin);await page.getByLabel('Password',{exact:true}).fill(fixture.password);await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('heading',{name:'Library',exact:true}).waitFor();
 await page.goto(`${fixture.origin}/?collection=${fixture.collection_id}&item=${fixture.item_id}`);
 let cancelledOnly=false,expectedOpusMedia=null;
 if(fixture.scenario){
  const openPreparation=async()=>{
   await clickAction(page,'Edit / attach files');
   const curator=page.getByRole('dialog',{name:/Edit entry/});
   await curator.getByRole('button',{name:'Music',exact:true}).click();
   const preparation=curator.getByRole('region',{name:'Prepare audio'});
   const batch=fixture.batch_id?preparation.locator('.preparation-batches article').filter({hasText:fixture.batch_id}):null;
   if(batch)await batch.waitFor();return {curator,preparation,batch};
  };
  let {curator,preparation,batch}=await openPreparation();
  if(fixture.scenario.startsWith('prepare-opus-')||fixture.scenario.startsWith('restored-opus-')){
   assert.ok([96,128,192].includes(fixture.bitrate));
   const label=`Opus ${fixture.bitrate} kbps`;
   if(fixture.scenario.startsWith('prepare-opus-'))await preparation.getByRole('button',{name:`Prepare ${label}`,exact:true}).click();
   const link=preparation.getByRole('link',{name:new RegExp(`^Download ${label} ·`)});
   await link.waitFor({timeout:60000});
   const downloadEvent=page.waitForEvent('download');await link.click();const download=await downloadEvent;
   assert.equal(await download.failure(),null);
   const file=path.join(scratch,`opus-${fixture.bitrate}.opus`);await download.saveAs(file);
   const rows=await(await page.request.get(`${fixture.origin}/api/v1/items/${fixture.item_id}/music-tracks`)).json();
   const row=rows.find(value=>value.profile?.codec==='opus'&&value.profile.bitrate_kbps===fixture.bitrate);
   assert.equal(row.state.status,'available');expectedOpusMedia=`/api/v1/items/${fixture.item_id}/representations/${row.state.representation_id}/${row.state.file_index}`;const bytes=await readFile(file);
   assert.equal(bytes.length,row.state.length);assert.equal(createHash('sha256').update(bytes).digest('hex'),row.state.digest.hex);
   assert.equal(rows.some(value=>value.profile?.codec==='flac'&&value.state.status==='available'),false,'FLAC masked Opus-only playback');
   await curator.getByRole('button',{name:'Close curator',exact:true}).click();
  }else if(fixture.scenario==='cancel-preparation'){
   await batch.getByRole('button',{name:'Cancel outstanding tracks',exact:true}).click();
   await expect(batch).toContainText('1 cancelled');
   const response=await page.request.get(`${fixture.origin}/api/v1/music-preparation-batches/${fixture.batch_id}`);
   assert.equal(response.status(),200);const outcome=await response.json();
   assert.equal(outcome.cancelled_by_user,true);assert.equal(outcome.cancelled,1);assert.equal(outcome.succeeded,0);
   await page.screenshot({path:path.join(scratch,'nas-cancelled-preparation.png'),fullPage:true});
   cancelledOnly=true;
  }else{
   if(fixture.scenario==='retry-failed-preparation'){
    await expect(batch).toContainText('1 failed');
    await batch.getByText('Track outcomes',{exact:true}).click();
    await expect(batch.getByText(/qualification: encoder failed after output/)).toBeVisible();
    await curator.getByRole('button',{name:'Close curator',exact:true}).click();
    ({curator,preparation,batch}=await openPreparation());
    await expect(batch).toContainText('1 failed');
   }else if(fixture.scenario==='retry-cancelled-preparation'){
    await expect(batch).toContainText('1 cancelled');
   }else assert.equal(fixture.scenario,'restarted-preparation');
   if(fixture.scenario!=='restarted-preparation')await batch.getByRole('button',{name:'Retry unfinished tracks',exact:true}).click();
   await expect(batch).toContainText('1/1 ready',{timeout:60000});
   await curator.getByRole('button',{name:'Close curator',exact:true}).click();
  }
 }
 if(!cancelledOnly){
 const tracks=page.getByRole('region',{name:'Album tracks'});const player=page.getByRole('region',{name:'Music player'});
 await tracks.getByRole('button',{name:'Play album',exact:true}).click();
 await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.1;}return false;});
 if(expectedOpusMedia){assert.equal(new URL(await player.locator('audio').evaluate(audio=>audio.currentSrc)).pathname,expectedOpusMedia);await expect.poll(()=>player.locator('audio').evaluate(audio=>audio.duration)).toBeCloseTo(3,1);}
 await player.getByRole('slider').fill('1');await expect.poll(()=>player.locator('audio').evaluate(audio=>audio.currentTime)).toBeGreaterThan(.9);
 await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 await page.getByText('Album details, evidence & files',{exact:true}).click();
 for(const [name,expected] of Object.entries(fixture.originals)){
  const link=page.locator('.entry-inspection').getByRole('link',{name,exact:true});const href=await link.getAttribute('href');assert.ok(href);
  const response=await page.request.get(new URL(href,fixture.origin).toString());assert.equal(response.status(),200);const bytes=await response.body();assert.equal(bytes.length,expected.bytes);assert.equal(createHash('sha256').update(bytes).digest('hex'),expected.sha256);
 }
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.screenshot({path:path.join(scratch,'nas-mobile.png'),fullPage:true});
 await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:path.join(scratch,'nas-desktop.png'),fullPage:true});
 await tracks.locator('.music-track').first().getByRole('button',{name:/^Play track /}).click();await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');if(audio){audio.loop=true;return !audio.paused&&audio.currentTime>.1;}return false;});
 const element=await player.locator('audio').elementHandle();await page.getByRole('button',{name:'Close details',exact:true}).click();assert.deepEqual(await element.evaluate(audio=>({connected:audio.isConnected,paused:audio.paused})),{connected:true,paused:false});await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 }
 assert.deepEqual(errors,[]);console.log(`${fixture.scenario||'container-host'} browser passed (Chrome ${await browser.version()}): trusted PEM/name/identity and ${cancelledOnly?'actual preparation cancellation':'playback/seek/exact originals/persistent playback/mobile layout'}. Artifacts: ${scratch}`);
}catch(error){console.error('Browser location:',page?.url());await page?.screenshot({path:path.join(scratch,'failure.png'),fullPage:true}).catch(()=>{});throw error;
}finally{await browser?.close();}
