// Browser regressions for chooser state and uncertain command outcomes. The real
// component uses controlled HTTP fixtures; archive effects have store/HTTP tests.
import {build} from 'esbuild';
import {chromium,expect} from '@playwright/test';
import {createServer} from 'node:http';
import path from 'node:path';
import assert from 'node:assert/strict';

const root=path.resolve(import.meta.dirname,'..');
const bundle=await build({stdin:{contents:`
 import React,{useState} from 'react';
 import {createRoot} from 'react-dom/client';
 import Chooser from './src/collection-list-chooser';
 function Harness(){const [open,setOpen]=useState(false);return <><button onClick={()=>setOpen(true)}>Open chooser</button>{open&&<Chooser userId="owner" collectionId="c" itemId="i" label="Album" onClose={()=>setOpen(false)} onOpen={()=>{}} onSessionExpired={()=>{}}/>}</>;}
 createRoot(document.getElementById('root')).render(<Harness/>);
`,resolveDir:root,loader:'tsx'},bundle:true,write:false,format:'iife',platform:'browser'});
const server=createServer((request,response)=>{
 response.setHeader('content-type',request.url==='/harness.js'?'text/javascript':'text/html');
 response.end(request.url==='/harness.js'?bundle.outputFiles[0].text:'<div id="root"></div><script src="/harness.js"></script>');
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const origin=`http://127.0.0.1:${server.address().port}`;
let browser;
const summary=(id,name,revision=1,entry_count=0)=>({id,name,revision,collection_id:'c',kind:'items',entry_count,can_edit:true});
const open=async page=>{await page.goto(origin);await page.getByRole('button',{name:'Open chooser'}).click();await page.getByRole('button',{name:'Append to list',exact:true}).waitFor();};
try{
 browser=await chromium.launch({executablePath:process.env.CHROME_PATH||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const page=await browser.newPage();page.setDefaultTimeout(10000);
 const writes=[];
 await page.route('**/api/v1/**',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(request.method()==='PUT'){writes.push(url.pathname);await route.fulfill({json:summary('a','Alpha',2,1)});return;}
  await route.fulfill({json:{items:url.searchParams.get('search')==='Beta'?[summary('b','Beta')]:[summary('a','Alpha')],next_cursor:null}});
 });
 await open(page);await page.getByLabel('Destination list').selectOption('a');
 await page.getByLabel('Find a list').fill('Beta');
 await expect(page.getByLabel('Destination list').locator('option')).toHaveText(['Create a new list','Alpha (0)','Beta (0)']);
 await expect(page.getByLabel('Destination list')).toHaveValue('a');
 await page.getByRole('button',{name:'Append to list',exact:true}).click();await page.getByText('Album added to Alpha.',{exact:true}).waitFor();
 assert.deepEqual(writes,['/api/v1/collections/c/lists/a']);
 assert.equal(await page.evaluate(()=>document.body.style.overflow),'hidden');
 await page.getByLabel('Find a list').focus();await page.keyboard.press('Shift+Tab');await expect(page.getByRole('button',{name:'Close',exact:true})).toBeFocused();
 await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).toHaveCount(0);await expect(page.getByRole('button',{name:'Open chooser'})).toBeFocused();
 assert.equal(await page.evaluate(()=>document.body.style.overflow),'');await page.close();

 const racing=await browser.newPage();racing.setDefaultTimeout(10000);
 let releaseOld,oldStarted,oldFinished;
 const started=new Promise(resolve=>oldStarted=resolve);
 const delayed=new Promise(resolve=>releaseOld=resolve);
 const finished=new Promise(resolve=>oldFinished=resolve);
 await racing.route('**/api/v1/**',async route=>{
  const url=new URL(route.request().url());
  if(url.searchParams.get('cursor')){
   oldStarted();await delayed;
   try{await route.fulfill({json:{items:[summary('old','Old page')],next_cursor:'old-cursor'}});}catch{/* Superseded fetches may already be aborted. */}finally{oldFinished();}
   return;
  }
  const filtered=url.searchParams.get('search')==='Beta';
  await route.fulfill({json:{items:filtered?[summary('b','Beta')]:[summary('a','Alpha')],next_cursor:filtered?null:'next'}});
 });
 await open(racing);await racing.getByRole('button',{name:'More lists'}).click();await started;
 await racing.getByLabel('Find a list').fill('Beta');await expect(racing.getByLabel('Destination list').locator('option')).toHaveText(['Create a new list','Beta (0)']);
 releaseOld();await finished;await expect(racing.getByLabel('Destination list').locator('option')).toHaveText(['Create a new list','Beta (0)']);
 await expect(racing.getByRole('button',{name:'More lists'})).toHaveCount(0);await racing.close();

 for(const reopen of [false,true]){
  const uncertain=await browser.newPage();uncertain.setDefaultTimeout(10000);
  let revision=1,appendCount=0;const receipts=new Map();const submissions=[];
  await uncertain.route('**/api/v1/**',async route=>{
   const request=route.request(),url=new URL(request.url());
   if(request.method()==='PUT'){
    const key=request.headers()['idempotency-key'],body=request.postData();submissions.push({key,body});
    if(receipts.has(key)){assert.equal(body,receipts.get(key).body);await route.fulfill({json:receipts.get(key).result});return;}
    assert.equal(JSON.parse(body).expected_revision,revision);
    appendCount++;revision++;
    const result=summary('a','Alpha',revision,appendCount);receipts.set(key,{body,result});
    if(appendCount===1){await route.fulfill({status:200,contentType:'application/json',body:'{"id":'});return;}
    await route.fulfill({json:result});return;
   }
   if(url.pathname.endsWith('/a')){await route.fulfill({json:{list:summary('a','Alpha',revision,appendCount)}});return;}
   await route.fulfill({json:{items:[summary('a','Alpha',revision,appendCount)],next_cursor:null}});
  });
  await open(uncertain);await uncertain.getByLabel('Destination list').selectOption('a');
  await uncertain.getByRole('button',{name:'Append to list',exact:true}).click();await uncertain.getByRole('alert').waitFor();
  if(reopen){await uncertain.getByRole('button',{name:'Close',exact:true}).click();await uncertain.reload();await uncertain.getByRole('button',{name:'Open chooser'}).click();}
  else{await uncertain.getByRole('button',{name:'Reload lists'}).click();}
  await expect(uncertain.getByLabel('Destination list').locator('option')).toHaveText(['Create a new list','Alpha (1)']);
  await expect(uncertain.getByLabel('Destination list')).toBeDisabled();
  await uncertain.getByRole('button',{name:'Retry addition',exact:true}).click();await uncertain.getByText('Album added to Alpha.',{exact:true}).waitFor();
  assert.equal(appendCount,1,'uncertain retry added a duplicate occurrence');assert.deepEqual(submissions[0],submissions[1]);
  await uncertain.getByRole('button',{name:'Append to list',exact:true}).click();
  await expect.poll(()=>appendCount).toBe(2);assert.notEqual(submissions[2].key,submissions[0].key,'intentional repeated append reused the earlier receipt');
  await uncertain.close();
 }

 // Creation has its own receipt: losing that response must not leave a
 // second empty list behind when the chooser reopens and resumes the addition.
 const creation=await browser.newPage();creation.setDefaultTimeout(10000);
 let creates=0,createdAppends=0;const createReceipts=new Map();const createAttempts=[];
 await creation.route('**/api/v1/**',async route=>{
  const request=route.request();
  if(request.method()==='POST'){
   const key=request.headers()['idempotency-key'],body=request.postData();createAttempts.push({key,body});
   if(createReceipts.has(key)){assert.equal(body,createReceipts.get(key).body);await route.fulfill({json:createReceipts.get(key).result});return;}
   creates++;createReceipts.set(key,{body,result:summary('new','New mix')});await route.fulfill({status:200,contentType:'application/json',body:'{"id":'});return;
  }
  if(request.method()==='PUT'){createdAppends++;await route.fulfill({json:summary('new','New mix',2,1)});return;}
  await route.fulfill({json:{items:creates?[summary('new','New mix')]:[],next_cursor:null}});
 });
 await open(creation);await creation.getByLabel('New list name').fill('New mix');await creation.getByRole('button',{name:'Append to list',exact:true}).click();await creation.getByRole('alert').waitFor();
 await creation.getByRole('button',{name:'Close',exact:true}).click();await creation.reload();await creation.getByRole('button',{name:'Open chooser'}).click();
 await expect(creation.getByLabel('New list name')).toHaveValue('New mix');await creation.getByRole('button',{name:'Retry addition',exact:true}).click();await creation.getByText('Album added to New mix.',{exact:true}).waitFor();
 assert.equal(creates,1);assert.equal(createdAppends,1);assert.deepEqual(createAttempts[0],createAttempts[1]);await creation.close();

 // A real conflict permits a revised command, even when the selected list is
 // filtered out of the first page. The exact read refreshes that destination.
 const conflict=await browser.newPage();conflict.setDefaultTimeout(10000);let revision=1,first=true;
 await conflict.route('**/api/v1/**',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(request.method()==='PUT'){
   if(first){first=false;revision=2;await route.fulfill({status:409,json:{error:{code:'revision_conflict',message:'conflict'}}});return;}
   assert.equal(JSON.parse(request.postData()).expected_revision,2);revision=3;await route.fulfill({json:summary('a','Alpha',revision,1)});return;
  }
  if(url.pathname.endsWith('/a')){await route.fulfill({json:{list:summary('a','Alpha',revision)}});return;}
  await route.fulfill({json:{items:url.searchParams.get('search')==='Beta'?[summary('b','Beta')]:[summary('a','Alpha',revision)],next_cursor:null}});
 });
 await open(conflict);await conflict.getByLabel('Destination list').selectOption('a');await conflict.getByLabel('Find a list').fill('Beta');await expect(conflict.getByLabel('Destination list').locator('option')).toHaveCount(3);
 await conflict.getByRole('button',{name:'Append to list',exact:true}).click();await conflict.getByRole('alert').waitFor();await conflict.getByRole('button',{name:'Reload lists'}).click();
 await expect(conflict.getByRole('button',{name:'Append to list',exact:true})).toBeEnabled();await conflict.getByRole('button',{name:'Append to list',exact:true}).click();await conflict.getByText('Album added to Alpha.',{exact:true}).waitFor();await conflict.close();
 // A delayed exact refresh must not restore a destination the user just
 // cleared while loading. The command stays bound to the visible choice.
 const changed=await browser.newPage();changed.setDefaultTimeout(10000);
 let changedRevision=1,releaseExact,exactStarted;
 const exactReady=new Promise(resolve=>exactStarted=resolve);
 const exactDelay=new Promise(resolve=>releaseExact=resolve);
 await changed.route('**/api/v1/**',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(request.method()==='PUT'){changedRevision=2;await route.fulfill({status:409,json:{error:{code:'revision_conflict',message:'conflict'}}});return;}
  if(url.pathname.endsWith('/a')){exactStarted();await exactDelay;await route.fulfill({json:{list:summary('a','Alpha',changedRevision)}});return;}
  await route.fulfill({json:{items:url.searchParams.get('search')==='Beta'?[summary('b','Beta')]:[summary('a','Alpha',changedRevision)],next_cursor:null}});
 });
 await open(changed);await changed.getByLabel('Destination list').selectOption('a');await changed.getByLabel('Find a list').fill('Beta');await expect(changed.getByLabel('Destination list').locator('option')).toHaveCount(3);
 await changed.getByRole('button',{name:'Append to list',exact:true}).click();await changed.getByRole('alert').waitFor();await changed.getByRole('button',{name:'Reload lists'}).click();await exactReady;
 await changed.getByLabel('Destination list').selectOption('');await changed.getByLabel('New list name').fill('Different destination');releaseExact();
 await expect(changed.getByLabel('Destination list').locator('option')).toHaveText(['Create a new list','Beta (0)']);await expect(changed.getByLabel('Destination list')).toHaveValue('');await expect(changed.getByLabel('New list name')).toHaveValue('Different destination');await changed.close();

 console.log('Collection list chooser passed: visible destination, obsolete pagination, shared modal focus/scroll/dismissal, uncertain receipt replay after reload/reopen, intentional repeats and filtered conflict recovery.');
}finally{if(browser)await browser.close();await new Promise(resolve=>server.close(resolve));}
