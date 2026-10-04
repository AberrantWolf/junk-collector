import assert from 'node:assert/strict';
import {expect} from '@playwright/test';
import path from 'node:path';
export async function collectionListsJourney(page,{album,hasFlac}){
 const nav=page.getByRole('navigation',{name:'Collection tasks'});const origin=new URL(page.url()).origin;
 const openAlbum=async()=>{await page.goto(`${origin}/?collection=${album.collection_id}&item=${album.id}`);await page.getByRole('region',{name:'Album tracks'}).getByRole('button',{name:'Add album to list',exact:true}).waitFor();};
 await openAlbum();const tracks=page.getByRole('region',{name:'Album tracks'});
 await tracks.getByRole('button',{name:'Add album to list',exact:true}).click();
 const chooser=page.getByRole('dialog',{name:'Add to list',exact:true});await chooser.getByLabel('New list name').fill('Evening mix');await chooser.getByRole('button',{name:'Append to list',exact:true}).click();await chooser.getByText(`${album.title} added to Evening mix.`,{exact:true}).waitFor();await chooser.getByRole('button',{name:'Open list',exact:true}).click();
 const lists=page.getByRole('region',{name:'Collection lists'});await lists.getByRole('heading',{name:'Evening mix',exact:true}).waitFor();const listId=new URL(page.url()).searchParams.get('list');assert.ok(listId);
 assert.equal(await lists.locator('.saved-list-entries li').count(),2);assert.match(await lists.locator('.saved-list-entries li').nth(0).innerText(),/Disc 1/);assert.match(await lists.locator('.saved-list-entries li').nth(1).innerText(),/Disc 2/);
 await openAlbum();await tracks.getByRole('button',{name:'Add disc to list',exact:true}).nth(1).click();await chooser.getByLabel('Destination list').selectOption(listId);await chooser.getByRole('button',{name:'Append to list',exact:true}).click();await chooser.getByRole('status').waitFor();await chooser.getByRole('button',{name:'Close',exact:true}).click();
 await tracks.getByRole('button',{name:'Add track to list',exact:true}).nth(0).click();await chooser.getByLabel('Destination list').selectOption(listId);await chooser.getByRole('button',{name:'Append to list',exact:true}).click();await chooser.getByRole('button',{name:'Open list',exact:true}).click();await expect(lists.locator('.saved-list-entries li')).toHaveCount(4);
 const ids=await lists.locator('.saved-list-entries li').evaluateAll(rows=>rows.map(row=>row.dataset.listEntryId));assert.equal(new Set(ids).size,4);
 await lists.locator('.saved-list-entries li').nth(0).getByRole('button',{name:/Move .* down/}).focus();await page.keyboard.press('Enter');await expect.poll(()=>lists.locator('.saved-list-entries li').first().getAttribute('data-list-entry-id')).toBe(ids[1]);
 await lists.locator('.saved-list-entries li').last().getByRole('button',{name:'Move to end',exact:true}).click();await lists.getByText('List saved.',{exact:true}).waitFor();
 await lists.locator('.saved-list-entries li').last().getByRole('button',{name:/Remove .* from list/}).click();await expect(lists.locator('.saved-list-entries li')).toHaveCount(3);
 await lists.getByText('List settings',{exact:true}).click();await lists.getByLabel('Rename list').fill('Night drive');await lists.getByRole('button',{name:'Save name',exact:true}).click();await lists.getByRole('heading',{name:'Night drive',exact:true}).waitFor();
 const saved=await lists.locator('.saved-list-entries li').evaluateAll(rows=>rows.map(row=>row.dataset.listEntryId));
 if(hasFlac){
  await lists.getByRole('button',{name:'Play playlist',exact:true}).click();await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentTime>.1;});
  const player=page.getByRole('region',{name:'Music player'});await player.locator('audio').evaluate(audio=>audio.loop=true);
  const element=await player.locator('audio').elementHandle();await nav.getByRole('button',{name:'Library',exact:true}).click();await page.waitForTimeout(300);assert.equal(await element.evaluate(audio=>audio.paused),false,'Lists navigation stopped playback');
  await nav.getByRole('button',{name:'Lists',exact:true}).click();await lists.getByRole('heading',{name:'Night drive',exact:true}).waitFor();await lists.getByRole('button',{name:'Shuffle play',exact:true}).click();await page.waitForFunction(()=>{const audio=document.querySelector('.music-player audio');return audio&&!audio.paused&&audio.currentTime>.1;});assert.deepEqual(await lists.locator('.saved-list-entries li').evaluateAll(rows=>rows.map(row=>row.dataset.listEntryId)),saved,'Shuffle changed saved order');
  await player.getByRole('button',{name:'Pause',exact:true}).click();await player.locator('audio').evaluate(audio=>audio.loop=false);
 }
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Lists overflow on mobile');await page.screenshot({path:path.resolve(import.meta.dirname,'../test-results/collection-lists-mobile.png'),fullPage:true});
 await page.reload();await lists.getByRole('heading',{name:'Night drive',exact:true}).waitFor();await expect(lists.locator('.saved-list-entries li')).toHaveCount(3);if(hasFlac)assert.equal(await page.locator('.music-player audio').evaluate(audio=>audio.paused),true,'Playlist reload autoplays');
 await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:path.resolve(import.meta.dirname,'../test-results/collection-lists-desktop.png'),fullPage:true});
 // A concurrent curator edit leaves the stale draft visible and cannot reorder the changed list.
 await page.evaluate(async({collection,list})=>{const header={"Content-Type":"application/json","Idempotency-Key":crypto.randomUUID()};const current=await(await fetch(`/api/v1/collections/${collection}/lists/${list}`)).json();const result=await fetch(`/api/v1/collections/${collection}/lists/${list}`,{method:'PUT',headers:header,body:JSON.stringify({expected_revision:current.list.revision,change:{action:'rename',name:'Night drive updated'}})});if(!result.ok)throw new Error(await result.text());},{collection:album.collection_id,list:listId});
 await lists.locator('.saved-list-entries li').first().getByRole('button',{name:'Move to end',exact:true}).click();await lists.getByRole('alert').getByText(/This list changed/).waitFor();await lists.getByRole('button',{name:'Reload list',exact:true}).click();await lists.getByRole('heading',{name:'Night drive updated',exact:true}).waitFor();
 await lists.getByRole('button',{name:'Back to lists',exact:true}).click();await lists.getByLabel('New list name').fill('Temporary list');await lists.getByRole('button',{name:'Create list',exact:true}).click();await lists.getByRole('heading',{name:'Temporary list',exact:true}).waitFor();await lists.getByText('List settings',{exact:true}).click();await lists.getByRole('button',{name:'Delete list',exact:true}).click();await lists.getByRole('button',{name:'Confirm delete list',exact:true}).click();await lists.getByLabel('New list name').waitFor();
 // Lose the response after a real committed append, then reload and replay
 // its receipt. A retry must not create another intentional-repeat occurrence.
 await openAlbum();await tracks.getByRole('button',{name:'Add track to list',exact:true}).nth(0).click();await chooser.getByLabel('Destination list').selectOption(listId);
 const count=()=>page.evaluate(async({collection,list})=>(await(await fetch(`/api/v1/collections/${collection}/lists/${list}`)).json()).list.entry_count,{collection:album.collection_id,list:listId});
 const before=await count();const attempts=[];let loseResponse=true;
 const endpoint=`**/api/v1/collections/${album.collection_id}/lists/${listId}`;
 await page.route(endpoint,async route=>{
  if(route.request().method()!=='PUT'){await route.continue();return;}
  attempts.push({key:route.request().headers()['idempotency-key'],body:route.request().postData()});
  if(loseResponse){loseResponse=false;const response=await route.fetch();assert.ok(response.ok());await route.fulfill({response,body:'{"id":'});}
  else await route.continue();
 });
 try{
  await chooser.getByRole('button',{name:'Append to list',exact:true}).click();await chooser.getByRole('alert').waitFor();assert.equal(await count(),before+1);
  await chooser.getByRole('button',{name:'Reload lists',exact:true}).click();await chooser.getByRole('button',{name:'Retry addition',exact:true}).click();await chooser.getByText(`Track 1 from ${album.title} added to Night drive updated.`,{exact:true}).waitFor();
  assert.equal(await count(),before+1,'receipt recovery duplicated the append');assert.deepEqual(attempts[0],attempts[1]);
 }finally{await page.unroute(endpoint);}
 await chooser.getByRole('button',{name:'Close',exact:true}).click();
 await page.goto(`${origin}/?collection=${album.collection_id}`);await page.getByRole('searchbox',{name:'Search collection'}).fill('pressing');
 console.log('Collection lists journey passed: album/disc/track append, repeated occurrences, keyboard reorder, removal, rename/delete, conflict reload, committed-response recovery, mobile, paused reload, playlist and shuffle playback.');
}
