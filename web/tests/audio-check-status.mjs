import {build} from 'esbuild';
import {chromium,expect} from './support/browser.mjs';
import {createServer} from 'node:http';
import path from 'node:path';
import assert from 'node:assert/strict';
const web=path.resolve(import.meta.dirname,'..');
const fixtures=[['Unchecked',[null]],['Passed',['verified']],['All passed',['verified','verified']],['Partial',['verified',null]],['Mismatch',['mismatched']],['Partial mismatch',['verified','mismatched']],['No data discs',['no_data','no_data']],['Mixed',['no_data',null]],['No data',['no_data']],['Limited',['incomplete_search']],['Ambiguous',['ambiguous_offsets']],['Unavailable',['unavailable']],['Unsupported',['unsupported']],['Empty',[]],['Reader',['verified']]];
const bundled=await build({stdin:{contents:`import React,{useState} from './src/runtime';import {createRoot} from 'react-dom/client';import AudioCheckStatus from './src/audio-check-status';function App(){const [opened,setOpened]=useState('');return <><p role="status">{opened}</p>{${JSON.stringify(fixtures)}.map(([title,statuses])=><section aria-label={title} key={title}><AudioCheckStatus item={{title,accuraterip:statuses.map((status,index)=>({source_id:String(index),status}))}} onOpen={title==='Reader'?undefined:()=>setOpened(title)}/></section>)}</>;}createRoot(document.getElementById('root')).render(<App/>);`,resolveDir:web,loader:'tsx'},bundle:true,write:false,format:'iife',jsxFactory:'React.createElement',jsxFragment:'React.Fragment',tsconfig:path.join(web,'tsconfig.json')});
const server=createServer((_request,response)=>{response.setHeader('Content-Type','text/html');response.end(`<div id="root"></div><script>${bundled.outputFiles[0].text}</script>`);});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));let browser;
try{
 browser=await chromium.launch({headless:true});const page=await browser.newPage();await page.goto(`http://127.0.0.1:${server.address().port}`);
 const label=name=>page.getByRole('region',{name,exact:true}).innerText().then(text=>text.trim());
 const outcomes=['Unchecked','Passed','Mismatch','No data','Limited','Ambiguous','Unavailable','Unsupported'];
 for(const name of outcomes)await expect(page.getByRole('region',{name,exact:true}).getByRole('button')).toBeVisible();
 const labels=await Promise.all(outcomes.map(label));assert.equal(new Set(labels).size,outcomes.length,'distinct outcomes must have distinct labels');
 // Incomplete or unsuccessful sets must never present the same outcome as a fully verified album.
 const verified=await label('Passed');for(const name of ['Unchecked','Mismatch','Partial','Partial mismatch','Mixed','Empty'])assert.notEqual(await label(name),verified,`${name} must not read as verified`);
 assert.notEqual(await label('All passed'),await label('Partial'),'a partly verified album must differ from a fully verified one');
 assert.notEqual(await label('Partial mismatch'),await label('Partial'),'a mismatch must not be hidden behind partial verification');assert.notEqual(await label('Partial mismatch'),await label('Mismatch'));
 // A read-only status presents the same outcome as an actionable one.
 assert.equal(await label('Reader'),verified);
 await expect(page.getByRole('region',{name:'Empty',exact:true}).getByRole('button')).toHaveCount(0);
 await expect(page.getByRole('region',{name:'Reader',exact:true}).getByRole('button')).toHaveCount(0);
 const partial=page.getByRole('region',{name:'Partial',exact:true}).getByRole('button');const perOriginal=await partial.getAttribute('title');assert.ok(perOriginal&&perOriginal!==await page.getByRole('region',{name:'Passed',exact:true}).getByRole('button').getAttribute('title'),'per-original detail must describe each original');await partial.focus();await page.keyboard.press('Enter');await expect(page.getByRole('status')).toHaveText('Partial');
 console.log('AccurateRip status: unchecked, verified, partial and distinct unsuccessful outcomes; empty/viewer read-only; keyboard shortcut targets the correct album.');
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve));}
