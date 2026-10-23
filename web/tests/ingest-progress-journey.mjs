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
 await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
 const release=holdProvider();
 try{
  await imports.getByRole('button',{name:'Start import',exact:true}).click();
  await imports.getByRole('heading',{name:'Import · 31 of 31 packages saved'}).waitFor();
  const intakeId=new URL(page.url()).searchParams.get('item');
  await expect.poll(()=>page.evaluate(async id=>{
   const rows=(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items;
   return rows.some(m=>m.import?.phase==='published')&&rows.some(m=>m.import?.work.some(w=>w.kind==='import_staging'));
  },intakeId),{timeout:60000}).toBe(true);
  assert.equal(await imports.getByText('Reselect original folder to resume the complete batch',{exact:true}).count(),0);
  const activity=imports.getByLabel('Current ingest activity');
  await expect(activity.filter({hasText:/Active.*Find metadata and artwork/}).first()).toBeVisible({timeout:180000});
  await expect(activity.filter({hasText:/Queued.*Find metadata and artwork/}).first()).toBeVisible({timeout:60000});
  await expect(activity.filter({hasText:'Shared download'}).first()).toBeVisible({timeout:60000});
  assert.equal(await activity.filter({hasText:'In progress'}).count(),0);
  const intake=new URL(page.url()).searchParams.get('item');
  const rows=await page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);
  assert.ok(rows.some(m=>m.import?.work.some(w=>w.state==='running')));
  assert.ok(rows.some(m=>m.import?.work.some(w=>w.state==='queued')));
  await page.screenshot({path:path.join(scratch,'ingest-activity-desktop.png'),fullPage:true});
  await imports.getByRole('button',{name:'Review import',exact:true}).first().click();
  await expect(imports.getByText('Copy files from inbox',{exact:true})).toBeVisible();
  await page.reload({waitUntil:'domcontentloaded'});
  await expect(imports.getByLabel('Current ingest activity').filter({hasText:/Active.*Find metadata and artwork/}).first()).toBeVisible({timeout:180000});
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await expect(imports.getByText(names[0],{exact:true})).toBeVisible();
  await imports.locator('.care-row').first().scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(scratch,'ingest-activity-mobile.png'),fullPage:true});
 }finally{release();await page.setViewportSize({width:1440,height:1000});}
 const intake=new URL(page.url()).searchParams.get('item');
 // Both release variants acquire artwork through the single paced provider
 // slot. This checks settlement of 31 packages, without imposing a two-minute
 // throughput requirement on the one-worker configuration.
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,intake),{timeout:240000}).toBe('completed');
 await expect(imports.getByLabel('Current ingest activity').filter({hasText:'Active'})).toHaveCount(0);
 const proposals=await page.evaluate(async id=>{
  const members=(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items;
  const entries=members.map(member=>member.import.target_item_id);
  const response=await fetch(`/api/v1/items/${entries[0]}/album-group-proposals?limit=32`);
  if(!response.ok)throw Error(await response.text());
  return {entries,peers:(await response.json()).items.map(item=>item.id)};
 },intake);
 assert.equal(new Set(proposals.entries).size,31,'Conflicting release disc placements retain reviewed grouping');
 assert.deepEqual(new Set(proposals.peers),new Set(proposals.entries.slice(1)),'Accepted album identities expose all grouping proposals');
 console.log('31-album real inbox activity: active/queued/shared provider stages, reload, mobile, terminal polling and accepted album grouping proposals passed.');
}
