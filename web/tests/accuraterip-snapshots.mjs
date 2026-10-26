// Real check-panel interactions with offline API fixtures; no provider requests.
import {build} from 'esbuild';
import {chromium,expect} from './support/browser.mjs';
import {createServer} from 'node:http';
import path from 'node:path';
import assert from 'node:assert/strict';
const web=path.resolve(import.meta.dirname,'..');
const queries={a:{track_count:18,accuraterip_id1:10,accuraterip_id2:20,cddb_id:30},b:{track_count:18,accuraterip_id1:11,accuraterip_id2:21,cddb_id:31},c:{track_count:18,accuraterip_id1:12,accuraterip_id2:22,cddb_id:32}};
const snapshot=(id,query,active=false)=>({active,snapshot:{id,provider:'org.junk.providers.accuraterip',created_at:'2026-10-09T00:00:00Z',parsed_observation:{schema_version:1,query,outcome:'data',response_count:1}}});
let fetched=false;const submitted=[];
const bundled=await build({stdin:{contents:`import React,{useState} from './src/runtime';import {createRoot} from 'react-dom/client';import ProcessingWorkspace from './src/processing-workspace';const capabilities=['org.junk.music.cd-structure','org.junk.music.accuraterip-verification'].map(id=>({id,availability:{state:'available'}}));const item={id:'album',collection_id:'music',domain:{module:'org.junk.music',payload:{}},presentation:{capabilities},captures:['a','b','c'].map(id=>({id,association:{item_id:'album'},representations:[{id,role:'preservation_master',format:id,files:[{name:id+'.toc'}]}]}))};function App(){const [error,setError]=useState('');return <><p role="alert">{error}</p><ProcessingWorkspace item={item} busy={false} setBusy={()=>{}} onError={error=>setError(String(error))} focusOperation="accuraterip"/></>;}createRoot(document.getElementById('root')).render(<App/>);`,resolveDir:web,loader:'tsx'},bundle:true,write:false,format:'iife',jsxFactory:'React.createElement',jsxFragment:'React.Fragment',tsconfig:path.join(web,'tsconfig.json'),plugins:[{name:'runtime',setup(b){b.onResolve({filter:/^@collector\/runtime$/},()=>({path:path.join(web,'src/runtime.ts')}));}}]});
const server=createServer(async(request,response)=>{
 response.setHeader('Content-Type','application/json');const url=new URL(request.url,'http://localhost');
 const send=value=>response.end(JSON.stringify(value));
 if(url.pathname==='/'){response.setHeader('Content-Type','text/html');response.end(`<div id="root"></div><script>${bundled.outputFiles[0].text}</script>`);return;}
 if(url.pathname.endsWith('/catalog-snapshots')){if(url.searchParams.has('cursor'))send({items:[snapshot('saved-a',queries.a),...(fetched?[snapshot('saved-c',queries.c)]:[])],next_cursor:null});else send({items:[snapshot('saved-b',queries.b,true)],next_cursor:'older'});return;}
 const match=url.pathname.match(/\/captures\/([abc])\/(.+)$/);
 if(match){const [,source,action]=match;
  if(action==='operations'&&request.method==='GET'){send({items:[{observation_id:'structure-'+source,current:true,result:source==='c'?{observations:[{result:'layout',tracks:[],disc_ids:{}}]}:{result:'layout',tracks:[],disc_ids:{}},operation:'org.junk.music.cd-structure',occurrences:1,last_checked_at:'2026-10-09T00:00:00Z',limitations:[]}],next_cursor:null});return;}
  if(action==='accuraterip-snapshots'){assert.equal(source,'c');let raw='';for await(const chunk of request)raw+=chunk;assert.equal(JSON.parse(raw).structure_observation_id,'structure-c');fetched=true;send({job_id:'fetch-c'});return;}
  let raw='';for await(const chunk of request)raw+=chunk;const command=JSON.parse(raw);if(command.snapshot_id===null)assert.equal(command.selection.snapshot_id,undefined,'An absent response must be omitted rather than submitted as an empty UUID');
  const expected='saved-'+source;
  if(action==='operations/plan'){const running=source==='a'&&submitted.length>0;send({state:running?'running':command.snapshot_id===expected?'ready':'blocked',command,files:[{file_index:0,label:source+'.toc'}],blockers:command.snapshot_id===expected?[]:['Choose a matching response'],accuraterip_query:queries[source],accuraterip_structure_observation_id:'structure-'+source,job:running?{id:'verify-a',kind:'processing_operation',state:'running',phase:'checking_accuraterip',progress_kind:'indeterminate',completed_units:0,total_units:999,cancellable:true}:null,result:null});return;}
  if(action==='operations'){submitted.push(command);assert.equal(command.snapshot_id,expected);send({job_id:'verify-'+source});return;}
 }
 if(url.pathname==='/api/v1/jobs/fetch-c'){send({state:'completed'});return;}
 response.statusCode=404;send({error:{message:'Unhandled fixture '+url.pathname}});
});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));let browser;
try{
 browser=await chromium.launch({headless:true});const page=await browser.newPage();const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.addInitScript(()=>sessionStorage.setItem('collector.scan.a',JSON.stringify({snapshotId:'saved-b'})));
 await page.goto(`http://127.0.0.1:${server.address().port}`);
 const docket=source=>page.locator('.import-docket').filter({has:page.locator('.docket-title',{hasText:source+'.toc ·'})});
 for(const source of ['a','b']){await expect(docket(source).getByLabel('Saved AccurateRip response')).toHaveValue('saved-'+source);await expect(docket(source).getByLabel('Saved AccurateRip response').locator('option')).toHaveCount(1);await expect(docket(source).getByRole('button',{name:'Run check',exact:true})).toBeEnabled();}
 await expect(docket('c').getByLabel('Saved AccurateRip response')).toHaveValue('');await expect(docket('c').getByText('No saved response matches this disc.',{exact:false})).toBeVisible();await expect(docket('c').getByRole('button',{name:'Run check',exact:true})).toBeDisabled();await expect(docket('c').getByRole('button',{name:'Fetch AccurateRip response',exact:true})).toBeVisible();await expect(docket('c').getByLabel('Source file')).toHaveValue('');await expect(docket('c').getByText(/UUID|invalid module data/)).toHaveCount(0);
 await docket('a').getByRole('button',{name:'Run check',exact:true}).click();await expect.poll(()=>submitted.length).toBe(1);assert.equal(submitted[0].snapshot_id,'saved-a');await expect(docket('a').getByRole('status')).toContainText('Computing AccurateRip checksums and checking offsets');await expect(docket('a').getByRole('progressbar')).not.toHaveAttribute('value');await expect(docket('a').getByRole('status')).not.toContainText('0 of');
 await docket('c').getByRole('button',{name:'Fetch AccurateRip response',exact:true}).click();await expect(docket('c').getByLabel('Saved AccurateRip response')).toHaveValue('saved-c');await expect(docket('c').getByRole('button',{name:'Run check',exact:true})).toBeEnabled();await expect(docket('a').getByLabel('Saved AccurateRip response')).toHaveValue('saved-a');await expect(docket('b').getByLabel('Saved AccurateRip response')).toHaveValue('saved-b');assert.deepEqual(errors,[]);
 console.log('AccurateRip responses: distinct discs with equal track counts, stale preference repaired, older-page match, missing response blocked with source choices and import-only fetch action, and fetch refresh selects each source independently.');
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve));}
