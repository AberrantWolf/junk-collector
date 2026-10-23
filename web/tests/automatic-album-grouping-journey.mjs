import {mkdir,writeFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {expect} from '@playwright/test';

export async function automaticAlbumGroupingJourney(page,{scratch,collectionId,restartServer}){
 const origin=new URL(page.url()).origin;
 const names=Array.from({length:4},(_,index)=>`Automatic soundtrack disc ${index+1}`);
 for(const [index,name] of names.entries()){
  const folder=path.join(scratch,'inbox',name);await mkdir(folder);
  await writeFile(path.join(folder,'disc.cue'),'FILE "disc.bin" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n');
  await writeFile(path.join(folder,'disc.bin'),Buffer.alloc((225+index)*2352,1));
 }
 await page.goto(`${origin}/imports?collection=${collectionId}`);
 const imports=page.getByRole('region',{name:'Imports workspace'});
 await imports.getByRole('combobox',{name:/^Source/}).selectOption('inbox');
 const chooser=imports.getByRole('region',{name:'Inbox package chooser'});
 await chooser.getByRole('searchbox').fill('Automatic soundtrack');
 for(const name of names)await chooser.getByLabel(name,{exact:true}).check();
 await imports.getByRole('button',{name:'Review packages',exact:true}).click();
 await imports.getByRole('heading',{name:'Review packages and groups'}).waitFor();
 await imports.getByRole('button',{name:'Start import',exact:true}).click();
 await imports.getByRole('heading',{name:'Import · 4 of 4 packages saved'}).waitFor();
 const intake=new URL(page.url()).searchParams.get('item');
 await expect.poll(()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}`)).json()).state,intake),{timeout:90000}).toBe('completed');
 const members=()=>page.evaluate(async id=>(await(await fetch(`/api/v1/intakes/${id}/members?limit=32`)).json()).items,intake);
 await expect.poll(async()=>new Set((await members()).map(member=>member.import.target_item_id)).size,{timeout:60000}).toBe(1);
 const rows=await members();const destination=rows[0].import.target_item_id;
 const album=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}`)).json()),destination);
 assert.equal(album.captures.length,4);assert.equal(album.specimens.length,4);
 const originals=await page.evaluate(async album=>Promise.all(album.captures.flatMap(capture=>capture.representations).filter(source=>source.role==='preservation_master').flatMap(source=>source.files.filter(file=>file.name.endsWith('.bin')).map(async file=>{
  const response=await fetch(`/api/v1/items/${album.id}/representations/${source.id}/${file.index}`);
  if(!response.ok)throw Error(await response.text());
  const bytes=new Uint8Array(await response.arrayBuffer());return {length:bytes.length,unchanged:bytes.every(value=>value===1)};
 }))),album);
 assert.deepEqual(originals.map(file=>file.length).sort((a,b)=>a-b),[225,226,227,228].map(sectors=>sectors*2352));
 assert.ok(originals.every(file=>file.unchanged),'Automatic grouping changed original bytes');
 if(process.env.GROUPING_AMBIGUITY){
  const metadata=await page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/metadata`)).json()),destination);
  assert.equal(metadata.display.resolution,'tentative');
  assert.equal(metadata.authority.selected_candidate??null,null);assert.equal(metadata.authority.automatic_candidate??null,null);
  assert.equal(new Set(metadata.candidates.filter(candidate=>candidate.current).map(candidate=>candidate.music_layout.album_id)).size,2);
 }
 const undo=await page.evaluate(async id=>(await(await fetch(`/api/v1/album-groups/${id}/undo`)).json()),album.merge_group);
 assert.equal(undo.group.automatic,true);assert.equal(undo.can_undo,true);
 const separated=undo.group.sources.length+1;
 await page.goto(`${origin}/?collection=${collectionId}`);
 await page.getByRole('searchbox',{name:'Search collection'}).fill(album.title);
 await expect(page.locator('.collection-row').filter({hasText:album.title})).toHaveCount(1);
 await page.locator('.collection-row').filter({hasText:album.title}).click();
 if(!process.env.GROUPING_AMBIGUITY)await expect(page.getByRole('region',{name:'Album tracks'}).locator('.music-track')).toHaveCount(4);
 // Old metadata links follow the canonical destination after the automatic merge.
 const old=await page.evaluate(async id=>{const status=await(await fetch(`/api/v1/album-groups/${id}/undo`)).json();return status.group.sources[0];},album.merge_group);
 await page.goto(`${origin}/library-care/metadata?collection=${collectionId}&item=${old}`);
 const history=page.getByRole('region',{name:'Album grouping history'});
 await expect(history.getByText(/Grouped automatically from matching MusicBrainz/)).toBeVisible();
 assert.equal(new URL(page.url()).searchParams.get('item'),destination);
 await page.goBack();if(!process.env.GROUPING_AMBIGUITY)await expect(page.getByRole('region',{name:'Album tracks'}).locator('.music-track')).toHaveCount(4);
 await page.goForward();await expect(history.getByText(/Grouped automatically from matching MusicBrainz/)).toBeVisible();
 if(process.env.GROUPING_AMBIGUITY){
  const review=page.getByRole('region',{name:'Review selected metadata'});
  await expect(review.getByRole('button',{name:'Use this album release',exact:true}).first()).toBeVisible();
  await expect(review.getByText('Another possible album',{exact:true}).first()).toBeVisible();
 }
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.screenshot({path:path.join(scratch,'automatic-grouping-mobile.png'),fullPage:true});
 if(process.env.GROUPING_AMBIGUITY){
  const readMetadata=()=>page.evaluate(async id=>(await(await fetch(`/api/v1/items/${id}/metadata`)).json()),destination);
  const review=page.getByRole('region',{name:'Review selected metadata'});
  await review.locator('article.candidate').filter({has:page.getByText('Bulk resolved album',{exact:true})}).first().getByRole('button',{name:'Use this album release',exact:true}).click();
  await expect.poll(async()=>(await readMetadata()).authority.selected_candidate?.provider_record_id).toBe('11111111-1111-4111-8111-111111111111');
  await expect(history.getByRole('button',{name:'Undo album grouping',exact:true})).toBeDisabled();
  await restartServer();await page.reload({waitUntil:'domcontentloaded'});
  await expect.poll(async()=>(await readMetadata()).display.resolution).toBe('selected');
  const recovered=await members();
  assert.equal(new Set(recovered.map(member=>member.import.target_item_id)).size,1);
  assert.ok(recovered.every(member=>member.import.outcome==='succeeded'),'Grouping must retain completed import outcomes after restart');
  console.log('Ambiguous MusicBrainz grouping passed: four discs grouped before an album choice, alternatives retained, exact original downloads, old links, mobile review, manual selection and restart persistence.');
  return;
 }
 await history.getByRole('button',{name:'Undo album grouping',exact:true}).click();
 await expect.poll(async()=>new Set((await members()).map(member=>member.import.target_item_id)).size).toBe(separated);
 await expect.poll(()=>new URL(page.url()).searchParams.has('item')).toBe(false);
 await restartServer();await page.reload({waitUntil:'domcontentloaded'});
 const metadata=page.getByRole('region',{name:'Metadata workspace'});
 await metadata.getByLabel('Show',{exact:true}).selectOption('all');
 await page.getByRole('searchbox',{name:'Search collection'}).fill('Bulk resolved album');
 await expect(metadata.locator('[data-item-id]')).toHaveCount(separated);
 assert.equal(new Set((await members()).map(member=>member.import.target_item_id)).size,separated);
 console.log('Automatic MusicBrainz grouping passed: four imported discs, one canonical album with all originals/copies/tracks, old links, mobile, guarded undo and restart separation.');
}
