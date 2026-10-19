import {mkdir,writeFile,readFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {expect} from '@playwright/test';

export async function intakeReviewJourney(page,{scratch,startProviderFixture,restartServer}){
 const nav=page.getByRole('navigation',{name:'Collection tasks'});
 const inbox=path.join(scratch,'inbox');
 const packageNames=['Review disc 1','Review disc 2'];
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
  await nav.getByRole('button',{name:'Imports',exact:true}).click();
  const imports=page.getByRole('region',{name:'Imports workspace'});
  await imports.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
  const chooser=imports.getByRole('region',{name:'Inbox package chooser'});
  await chooser.getByLabel(packageNames[0],{exact:true}).waitFor();
  assert.equal(await chooser.getByLabel('#recycle',{exact:true}).count(),0);
  assert.equal(await chooser.getByLabel('@eaDir',{exact:true}).count(),0);
  for(const name of packageNames)await chooser.getByLabel(name,{exact:true}).check();
  await imports.getByRole('button',{name:'Review packages',exact:true}).click();
  await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
  for(const [index,name] of packageNames.entries()){
   const row=imports.locator('.care-row').filter({has:page.getByText(name,{exact:true})});
   await row.getByText('Files and analysis',{exact:true}).click();
   const anchor=row.getByRole('combobox',{name:`Analysis anchor for ${name}`});
   assert.equal(await anchor.locator('option').count(),3,'Both CUE and Redumper anchors must remain selectable');
   await row.getByLabel(/Curators only: .unknown/).check();
   await row.getByLabel('Album group',{exact:true}).fill('Shared review group');
   await expect(imports.getByRole('button',{name:'Start import',exact:true})).toBeDisabled();
   if(manual)await row.getByLabel('Manual album title (optional)',{exact:true}).fill('My explicit review title');
   await row.getByLabel('Physical copy label',{exact:true}).fill(manual?`Purchased copy ${index+1}`:'Purchased copy A');
   const cue=await anchor.locator('option').filter({hasText:'BIN/CUE'}).getAttribute('value');
   await anchor.selectOption(cue);
   await expect(row.getByText(/analysis available/)).toBeVisible();
   assert.equal(await row.getByLabel(/Curators only: .unknown/).isChecked(),true,'Replanning lost reviewed file privacy');
   assert.equal(await row.getByLabel('Physical copy label',{exact:true}).inputValue(),manual?`Purchased copy ${index+1}`:'Purchased copy A');
   assert.equal(await row.getByLabel('Manual album title (optional)',{exact:true}).inputValue(),manual?'My explicit review title':'');
   assert.equal(await row.getByText(/SYNO@EAStream|unused.cue|Curators only: .DS_Store/).count(),0);
  }
  // Draft choices survive a real page reload before submission.
  await page.reload({waitUntil:'domcontentloaded'});
  await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
  await expect(imports.getByLabel('Album group',{exact:true}).first()).toHaveValue('Shared review group');
  await page.screenshot({path:path.join(scratch,`review-${manual?'manual':'automatic'}-desktop.png`),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Import review overflows on mobile');
  await page.screenshot({path:path.join(scratch,`review-${manual?'manual':'automatic'}-mobile.png`),fullPage:true});
  await imports.getByRole('button',{name:'Start import',exact:true}).focus();await page.keyboard.press('Enter');
  await page.setViewportSize({width:1440,height:1000});
  await expect.poll(()=>new URL(page.url()).searchParams.get('item')).toBeTruthy();
  const intakeId=new URL(page.url()).searchParams.get('item');
  await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,intakeId),{timeout:60000}).toBe('completed');
  const members=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intakeId);
  assert.equal(members.length,2);
  assert.equal(new Set(members.map(m=>m.import.target_item_id)).size,1);
  const itemId=members[0].import.target_item_id;
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
  await restartServer();
  await page.reload({waitUntil:'domcontentloaded'});
  await imports.getByRole('heading',{name:'Import · 2 of 2 packages saved'}).waitFor();
  const recovered=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),itemId);
  assert.equal(recovered.title,album.title);assert.equal(recovered.specimens.length,album.specimens.length);assert.equal(recovered.captures.length,2);
  await imports.getByRole('button',{name:'Back to imports',exact:true}).click();
 }
 console.log('HL-07A passed: real inbox anchor selection, retained privacy/drafts, automatic/manual titles and artwork, one-copy/separate-copy grouping, housekeeping exclusion and restart.');
}
