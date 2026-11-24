import {mkdir,writeFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {expect} from '@playwright/test';

export async function ingestProgressJourney(page,{scratch,holdProvider,collectionId}){
 const imports=page.getByRole('region',{name:'Imports workspace'});
 const names=Array.from({length:31},(_,i)=>`Activity album ${String(i+1).padStart(2,'0')}`);
 for(const [index,name] of names.entries()){
  const folder=path.join(scratch,'inbox',name);await mkdir(folder);
  await writeFile(path.join(folder,'disc.cue'),'FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n');
  await writeFile(path.join(folder,'disc.bin'),Buffer.alloc((10+(index%2?index:0))*2352,1));
 }
 await page.goto(new URL(`/imports?collection=${collectionId}`,page.url()).href);
 await imports.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
 const chooser=imports.getByRole('region',{name:'Inbox package chooser'});
 await chooser.getByRole('searchbox').fill('Activity album');
 for(const name of names)await chooser.getByLabel(name,{exact:true}).check();
 await imports.getByRole('button',{name:'Review packages',exact:true}).click();
 await imports.getByRole('button',{name:'Start import',exact:true}).waitFor();
 const release=holdProvider();
 try{
  await imports.getByRole('button',{name:'Start import',exact:true}).click();
  // Inbox imports open Review > In progress; this journey watches the batch page.
  await expect.poll(()=>new URL(page.url()).searchParams.get('intake')).toBeTruthy();
  const intakeId=new URL(page.url()).searchParams.get('intake');
  await page.goto(new URL(`/imports?collection=${new URL(page.url()).searchParams.get('collection')}&item=${intakeId}`,page.url()).href);
  await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).package_count,intakeId)).toBe(31);
  await expect.poll(()=>page.evaluate(async id=>{
   const rows=(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items;
   return rows.some(m=>m.import?.phase==='published')&&rows.some(m=>m.import?.work.some(w=>w.kind==='import_staging'));
  },intakeId),{timeout:60000}).toBe(true);
  // Inbox imports never ask for a browser upload to be resumed.
  assert.equal(await imports.getByLabel('Reselect folder to resume upload',{exact:true}).count(),0);
  const members=()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intakeId);
  assert.ok((await members()).every(m=>!m.import?.permissions.can_resume_upload));
  // While the provider is held, metadata work is running for one package, queued for others, and shared between identical discs.
  const metadataWork=async state=>(await members()).some(m=>m.import?.work.some(w=>w.kind==='metadata_resolution'&&w.state===state));
  await expect.poll(()=>metadataWork('running'),{timeout:180000}).toBe(true);
  await expect.poll(()=>metadataWork('queued'),{timeout:60000}).toBe(true);
  await expect.poll(async()=>(await members()).some(m=>m.import?.work.some(w=>w.shared)),{timeout:60000}).toBe(true);
  const activity=imports.getByLabel('Current ingest activity');
  await expect(activity.locator('.ingest-work-running').first()).toBeVisible();
  await expect(activity.locator('.ingest-work-queued').first()).toBeVisible();
  const rows=await members();
  assert.ok(rows.some(m=>m.import?.work.some(w=>w.state==='running')));
  assert.ok(rows.some(m=>m.import?.work.some(w=>w.state==='queued')));
  await page.screenshot({path:path.join(scratch,'ingest-activity-desktop.png'),fullPage:true});
  // Each package opens in Review, where its steps and destination can be fixed.
  const batchUrl=page.url();
  await imports.getByRole('button',{name:'Open in Review',exact:true}).first().click();
  const review=page.getByRole('region',{name:'Selected import',exact:true});
  const reviewedPackage=review.locator('.ingest-detail > summary');
  await expect(reviewedPackage).toContainText(names[0]);
  await expect(page.getByRole('region',{name:'Imports in progress',exact:true}).locator('.review-row').first()).toBeVisible();
  // Every workflow step reported by the server is listed, including the inbox copy.
  const steps=(await page.evaluate(async id=>(await(await fetch(`/api/v1/ingests/${id}`)).json()).steps,rows[0].import_id));
  assert.ok(steps.some(step=>step.step==='staging'));
  await expect(review.locator('.ingest-steps > div')).toHaveCount(steps.length);
  await page.screenshot({path:path.join(scratch,'active-import-review-desktop.png')});
  const correction=review.locator('details:not(.ingest-detail)').filter({has:page.locator('[aria-label="Destination entry"]')}).locator(':scope > summary');
  await expect(correction).toBeVisible({timeout:60000});
  await correction.click();
  await review.getByRole('region',{name:'Destination entry',exact:true}).getByRole('button',{name:'Leave unattached',exact:true}).click();
  await expect(review.getByRole('button',{name:'Save destination',exact:true})).toBeVisible();
  async function confirmDiscard(action,accept){
   const prompt=page.waitForEvent('dialog',{timeout:5000});const actionDone=action();
   const dialog=await prompt;assert.equal(dialog.type(),'confirm');
   if(accept)await dialog.accept();else await dialog.dismiss();await actionDone;
  }
  // Leaving with an unsaved destination asks first, and refusing keeps the
  // draft across polling.
  const back=review.getByRole('button',{name:'Back to the list',exact:true});
  await confirmDiscard(()=>back.click(),false);
  await expect(review).toBeVisible();
  await page.waitForResponse(response=>response.request().method()==='GET'&&response.url().endsWith(`/ingests/${rows[0].import_id}`));
  await expect(review.getByRole('button',{name:'Save destination',exact:true})).toBeVisible();
  await confirmDiscard(()=>back.click(),true);
  await expect(review).toHaveCount(0);
  await page.setViewportSize({width:390,height:844});
  await page.getByRole('region',{name:'Imports in progress',exact:true}).locator('.review-row-main').filter({hasText:names[1]}).click();
  await expect(reviewedPackage).toContainText(names[1]);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Import review overflows on mobile');
  await page.screenshot({path:path.join(scratch,'active-import-review-mobile.png')});
  await page.setViewportSize({width:1440,height:1000});
  await page.goto(batchUrl);
  await page.reload({waitUntil:'domcontentloaded'});
  await expect(imports.getByLabel('Current ingest activity').locator('.ingest-work-running').first()).toBeVisible({timeout:180000});
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await expect(imports.getByText(names[0],{exact:true})).toBeVisible();
  await imports.locator('.care-row').first().scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(scratch,'ingest-activity-mobile.png'),fullPage:true});
 }finally{release();await page.setViewportSize({width:1440,height:1000});}
 const intake=new URL(page.url()).searchParams.get('item');
 console.log('Provider released; waiting for the 31 imports to settle.');
 // Both release variants acquire artwork through the single paced provider
 // slot. This checks settlement of 31 packages, without imposing a two-minute
 // throughput requirement on the one-worker configuration.
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,intake),{timeout:240000}).toBe('completed');
 await expect(imports.getByLabel('Current ingest activity').locator('.ingest-work-running')).toHaveCount(0);
 const entries=await page.evaluate(async id=>
  (await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items.map(member=>member.import.target_item_id),intake);
 assert.equal(new Set(entries).size,31,'Conflicting release disc placements retain reviewed grouping');
 console.log('31-album real inbox activity: active/queued/shared provider stages, reload, mobile, terminal polling and preserved reviewed grouping passed.');
}
