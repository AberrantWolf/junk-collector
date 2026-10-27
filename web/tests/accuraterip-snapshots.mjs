// One server-owned workflow per disc; the browser submits no provider/processing pipeline.
import {build} from 'esbuild';
import {chromium,expect} from './support/browser.mjs';
import {createServer} from 'node:http';
import {readFile,mkdtemp} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
const web=path.resolve(import.meta.dirname,'..'),requests=[],states=new Map(),artifacts=await mkdtemp(path.join(tmpdir(),'collector-cd-check-ui-'));
const bundled=await build({stdin:{contents:`import React,{useState} from './src/runtime';import {createRoot} from 'react-dom/client';import Check from './src/accuraterip-check';function App(){const [open,setOpen]=useState(true);return <><button onClick={()=>setOpen(v=>!v)}>Toggle checks</button>{open&&['a','b'].map(source=><article key={source} aria-label={'Disc '+source}><Check itemId="album" sourceId={source}/></article>)}</>;}createRoot(document.getElementById('root')).render(<App/>);`,resolveDir:web,loader:'tsx'},bundle:true,write:false,format:'iife',jsxFactory:'React.createElement',jsxFragment:'React.Fragment',tsconfig:path.join(web,'tsconfig.json'),plugins:[{name:'runtime',setup(b){b.onResolve({filter:/^@collector\/runtime$/},()=>({path:path.join(web,'src/runtime.ts')}));}}]});
const server=createServer(async(req,res)=>{res.setHeader('Content-Type','application/json');const url=new URL(req.url,'http://localhost'),send=v=>res.end(JSON.stringify(v));
 if(url.pathname==='/'){res.setHeader('Content-Type','text/html');res.end(`<meta name="viewport" content="width=device-width,initial-scale=1"><style>${await readFile(path.join(web,'dist/app.css'),'utf8')}</style><div id="root"></div><script>${bundled.outputFiles[0].text}</script>`);return;}
 if(url.pathname==='/api/v1/items/album/accuraterip-checks'){send([...states.values()]);return;}
 if(url.pathname==='/api/v1/items/album/accuraterip-check'){let raw='';for await(const chunk of req)raw+=chunk;const body=JSON.parse(raw);requests.push({body,key:req.headers['idempotency-key']});assert.deepEqual(Object.keys(body),['representation_id']);const id=body.representation_id;states.set(id,{job:{id,state:'queued',phase:'waiting_provider',cancellable:true},sources:[id],result:null});send({job_id:id});return;}
 if(url.pathname==='/api/v1/jobs/b/cancel'){const v=states.get('b');v.job={...v.job,state:'cancelled',phase:'cancelled',cancellable:false};send({});return;}
 res.statusCode=404;send({error:{message:'Unexpected client pipeline request: '+url.pathname}});
});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));let browser;
try{browser=await chromium.launch();const page=await browser.newPage({viewport:{width:1200,height:900}}),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(`http://127.0.0.1:${server.address().port}`);const a=page.getByRole('article',{name:'Disc a'}),b=page.getByRole('article',{name:'Disc b'});
 await a.getByRole('button',{name:'Check this disc with AccurateRip',exact:true}).click();await expect(a.getByText('Waiting for provider download',{exact:true})).toBeVisible();assert.equal(requests.length,1);assert.ok(requests[0].key);await expect(b.getByRole('button',{name:'Check this disc with AccurateRip',exact:true})).toBeEnabled();
 await page.getByRole('button',{name:'Toggle checks'}).click();await page.getByRole('button',{name:'Toggle checks'}).click();await expect(a.getByRole('button',{name:'Checking disc with AccurateRip…'})).toBeDisabled();assert.equal(requests.length,1,'reopen must resume polling without resubmitting');
 states.get('a').job={id:'a',state:'completed',phase:'completed',cancellable:false};states.get('a').result={confirmed:true,discs:[{source_id:'a',status:'verified',observation_id:'a-result'}]};await expect(a.getByText('AccurateRip: Verified',{exact:true})).toBeVisible();
 await b.getByRole('button',{name:'Check this disc with AccurateRip',exact:true}).click();await b.getByRole('button',{name:'Cancel audio check',exact:true}).click();await expect(b.getByText('Audio check cancelled.',{exact:true})).toBeVisible();await expect(a.getByText('AccurateRip: Verified',{exact:true})).toBeVisible();
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);await page.screenshot({path:path.join(artifacts,'mobile.png'),fullPage:true});assert.deepEqual(errors,[]);console.log('One-click CD UI passed: independent discs, one command, server-driven provider progress, reopen polling, confirmed result, cancellation, mobile. Artifacts: '+artifacts);
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve));}
