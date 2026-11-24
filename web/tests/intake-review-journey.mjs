import {mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {expect} from '@playwright/test';

export async function inboxSelectorJourney(page,{scratch}){
 const names=Array.from({length:52},(_,i)=>`Chooser ${String(i).padStart(2,'0')}${i===0?' '+ 'LongFolderName'.repeat(12):''}`);
 for(const name of names){const folder=path.join(scratch,'inbox',name);await mkdir(folder);await writeFile(path.join(folder,'note.txt'),'Chooser fixture');}
 const chooser=page.getByRole('region',{name:'Inbox package chooser'});
 try{
  await page.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
  const rows=chooser.locator('.inbox-package-results input');
  await expect(rows).toHaveCount(50);
  const checked=chooser.locator('.inbox-package-results input:checked');
  const all=chooser.getByRole('checkbox',{name:'Select all shown',exact:true});
  await all.check();await expect(checked).toHaveCount(50);await expect(page.getByRole('button',{name:'Review packages',exact:true})).toBeEnabled();
  await chooser.getByRole('button',{name:'More packages',exact:true}).click();await expect(rows).toHaveCount(52);await expect(checked).toHaveCount(50);
  assert.equal(await all.evaluate(node=>node.indeterminate),true,'Newly loaded rows must not be silently selected');
  await all.focus();await page.keyboard.press('Space');await expect(checked).toHaveCount(52);
  await chooser.getByLabel(names[0],{exact:true}).uncheck();await expect(checked).toHaveCount(51);
  assert.equal(await all.evaluate(node=>node.indeterminate),true);
  await all.check();await expect(rows.first()).toBeChecked();await expect(checked).toHaveCount(52);
  // Changing the selection of a filtered view retains selections hidden by the filter.
  await chooser.getByLabel('Search inbox packages').fill('Chooser 51');await expect(rows).toHaveCount(1);
  await all.uncheck();await expect(rows.first()).not.toBeChecked();
  await chooser.getByLabel('Search inbox packages').fill('');await expect(rows).toHaveCount(50);await expect(checked).toHaveCount(50);
  await chooser.getByLabel('Search inbox packages').fill('Chooser 51');await expect(rows).toHaveCount(1);await expect(rows.first()).not.toBeChecked();
  await all.check();await expect(rows.first()).toBeChecked();
  await chooser.getByRole('button',{name:'Clear selection',exact:true}).click();await expect(checked).toHaveCount(0);
  await expect(page.getByRole('button',{name:'Review packages',exact:true})).toBeDisabled();
  await chooser.getByLabel('Search inbox packages').fill('');await expect(rows).toHaveCount(50);await expect(checked).toHaveCount(0);
  for(const [width,height,label] of [[1440,1000,'desktop'],[390,844,'mobile']]){
   await page.setViewportSize({width,height});await chooser.getByLabel(names[1],{exact:true}).check();
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Inbox chooser overflows');
   await chooser.screenshot({path:path.join(scratch,`inbox-selector-${label}.png`)});
  }
 }finally{
  await page.setViewportSize({width:1440,height:1000});
  for(const name of names)await rm(path.join(scratch,'inbox',name),{recursive:true,force:true});
  await page.reload({waitUntil:'domcontentloaded'});
 }
 console.log('Inbox selector: paginated bulk selection, mixed state, keyboard, filtered selection retention/clear, desktop/mobile alignment and long names passed.');
}

export async function intakeReviewJourney(page,{scratch,startProviderFixture,restartServer}){
 const nav=page.getByRole('navigation',{name:'Collection tasks'});
 const inbox=path.join(scratch,'inbox');
 const packageNames=['Review disc 1','Review disc 2'];let cleanupFixture;
 for(const [index,name] of packageNames.entries()){
  const root=path.join(inbox,name);
  for(const dir of ['@eaDir/disc.bin','#recycle/old','notes'])await mkdir(path.join(root,dir),{recursive:true});
  await writeFile(path.join(root,'disc.cue'),'FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n');
  await writeFile(path.join(root,'disc.bin'),Buffer.alloc((225+index)*2352,1));
  await writeFile(path.join(root,'disc.toc'),'Opaque alternate Redumper view, preserved without interpreting it.');
  await writeFile(path.join(root,'.unknown'),`Original hidden note ${index}`);
  await writeFile(path.join(root,'notes','@eaDir'),'A meaningful file with a directory-like name');
  for(const name of ['.DS_Store','@eaDir/disc.bin/SYNO@EAStream','#recycle/old/unused.cue'])await writeFile(path.join(root,name),'housekeeping');
 }
 for(const dir of ['#recycle','@eaDir'])await mkdir(path.join(inbox,dir),{recursive:true});
 startProviderFixture();
 for(const manual of [false,true]){
  await nav.getByRole('button',{name:'Add to library',exact:true}).click();
  const imports=page.getByRole('region',{name:'Imports workspace'});
  await imports.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
  const chooser=imports.getByRole('region',{name:'Inbox package chooser'});
  await chooser.getByLabel(packageNames[0],{exact:true}).waitFor();
  assert.equal(await chooser.getByLabel('#recycle',{exact:true}).count(),0);
  assert.equal(await chooser.getByLabel('@eaDir',{exact:true}).count(),0);
  for(const name of packageNames)await chooser.getByLabel(name,{exact:true}).check();
  await imports.getByRole('button',{name:'Review packages',exact:true}).click();
  await imports.getByRole('button',{name:'Start import',exact:true}).waitFor();
  for(const [index,name] of packageNames.entries()){
   const row=imports.locator('.care-row').filter({has:page.getByText(name,{exact:true})});
   const anchor=row.getByRole('combobox',{name:`Main file for ${name}`});
   await row.locator('details').filter({has:page.getByLabel(`Main file for ${name}`,{exact:true})}).locator(':scope > summary').click();
   assert.equal(await anchor.locator('option').count(),3,'Both CUE and Redumper anchors must remain selectable');
   await row.getByLabel(/Curators only: .unknown/).check();
   await row.getByLabel('Album group',{exact:true}).fill('Shared review group');
   await expect(imports.getByRole('button',{name:'Start import',exact:true})).toBeDisabled();
   if(manual)await row.getByLabel('Manual album title (optional)',{exact:true}).fill('My explicit review title');
   await row.getByLabel('Physical copy label',{exact:true}).fill(manual?`Purchased copy ${index+1}`:'Purchased copy A');
   const cue=await anchor.locator('option').filter({hasText:'disc.cue'}).getAttribute('value');
   const replanned=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/v1/ingests/plan'&&response.request().method()==='POST');
   await anchor.selectOption(cue);
   const plan=(await(await replanned).json()).packages[0];
   assert.equal(plan.analysis_available,true,'Choosing the CUE anchor must make the package analysable');
   assert.equal(plan.anchors.find(choice=>choice.selected)?.file_index,Number(cue));
   await expect(anchor).toHaveValue(cue);await expect(anchor).toBeEnabled();
   assert.equal(await row.getByLabel(/Curators only: .unknown/).isChecked(),true,'Replanning lost reviewed file privacy');
   assert.equal(await row.getByLabel('Physical copy label',{exact:true}).inputValue(),manual?`Purchased copy ${index+1}`:'Purchased copy A');
   assert.equal(await row.getByLabel('Manual album title (optional)',{exact:true}).inputValue(),manual?'My explicit review title':'');
   assert.equal(await row.getByText(/SYNO@EAStream|unused\.cue|\.DS_Store/).count(),0);
  }
  // Draft choices survive a real page reload before submission.
  await page.reload({waitUntil:'domcontentloaded'});
  await imports.getByRole('button',{name:'Start import',exact:true}).waitFor();
  await expect(imports.getByLabel('Album group',{exact:true}).first()).toHaveValue('Shared review group');
  await page.screenshot({path:path.join(scratch,`review-${manual?'manual':'automatic'}-desktop.png`),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Import review overflows on mobile');
  await page.screenshot({path:path.join(scratch,`review-${manual?'manual':'automatic'}-mobile.png`),fullPage:true});
  await imports.getByRole('button',{name:'Start import',exact:true}).focus();await page.keyboard.press('Enter');
  await page.setViewportSize({width:1440,height:1000});
  // An inbox import has nothing to upload, so it opens Review > In progress
  // for this batch.
  await expect.poll(()=>new URL(page.url()).searchParams.get('intake')).toBeTruthy();
  const intakeId=new URL(page.url()).searchParams.get('intake');
  assert.equal(new URL(page.url()).pathname,'/review');
  await expect(page.getByRole('region',{name:'Review workspace'}).getByRole('button',{name:'In progress',exact:true})).toHaveAttribute('aria-current','page');
  await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,intakeId),{timeout:60000}).toBe('completed');
  const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intakeId);
  assert.equal(members.length,2);
  assert.equal(new Set(members.map(m=>m.import.target_item_id)).size,1);
  const itemId=members[0].import.target_item_id;cleanupFixture={collectionId:members[0].import.collection_id,importId:members[0].import.import_id,itemId,source:path.join(inbox,packageNames[0])};
  const album=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),itemId);
  assert.equal(album.title,manual?'My explicit review title':'Bulk resolved album');
  assert.ok(album.primary_artwork,'Automatic artwork did not reach the grouped album');
  assert.equal(album.specimens.length,manual?2:1);
  assert.equal(album.captures.length,2);
  for(const [index,member] of members.entries()){
   assert.equal(member.plan.files.length,5);
   const files=member.plan.files;
   assert.equal(files.find(f=>f.name==='.unknown').visibility,'curators');
   const capture=album.captures.find(c=>c.id===member.import.capture_id);
   const representation=capture.representations.find(r=>r.role==='preservation_master');
   for(const file of representation.files){
    const original=await readFile(path.join(inbox,packageNames[index],file.name));
    const downloaded=await page.evaluate(async ({item,representation,index})=>{const r=await fetch(`/api/v1/items/${item}/representations/${representation}/${index}`);return {status:r.status,bytes:Array.from(new Uint8Array(await r.arrayBuffer()))};},{item:itemId,representation:representation.id,index:file.index});
    assert.equal(downloaded.status,200);assert.deepEqual(Buffer.from(downloaded.bytes),original);
   }
  }
  if(!manual){
   // Look at the real Review screens for this import.
   const reviewBase=new URL(`/review?collection=${members[0].import.collection_id}`,page.url());
   await page.goto(`${reviewBase}&tab=all&item=${itemId}`);
   const panel=page.getByRole('region',{name:'Review selected entry',exact:true});
   await panel.getByRole('heading',{level:2}).waitFor();
   await page.screenshot({path:path.resolve(import.meta.dirname,'../test-results/review-entry-desktop.png'),fullPage:true});
   await page.setViewportSize({width:390,height:844});
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Review panel overflows on mobile');
   await page.screenshot({path:path.resolve(import.meta.dirname,'../test-results/review-entry-mobile.png'),fullPage:true});
   await page.setViewportSize({width:1440,height:1000});
   await page.goto(`${reviewBase}&tab=added`);
   await page.getByRole('region',{name:'Added recently',exact:true}).locator('.review-row').first().waitFor();
   await page.screenshot({path:path.resolve(import.meta.dirname,'../test-results/review-added-desktop.png'),fullPage:true});
  }
  await restartServer();
  await page.goto(new URL(`/imports?collection=${members[0].import.collection_id}&item=${intakeId}`,page.url()).href,{waitUntil:'domcontentloaded'});
  await expect(imports.getByRole('button',{name:'Open in Review',exact:true})).toHaveCount(2);
  assert.equal((await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()),intakeId)).package_count,2);
  const recovered=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),itemId);
  assert.equal(recovered.title,album.title);assert.equal(recovered.specimens.length,album.specimens.length);assert.equal(recovered.captures.length,2);
  await imports.getByRole('button',{name:'Back to Add to library',exact:true}).click();
 }
 console.log('HL-07A passed: real inbox anchor selection, retained privacy/drafts, automatic/manual titles and artwork, one-copy/separate-copy grouping, housekeeping exclusion and restart.');
 return cleanupFixture;
}
