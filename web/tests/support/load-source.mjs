// Bundles one web/src module for plain Node and returns its exports.
// Unit tests must not quietly start depending on the browser or the network, so
// the bundle is refused when it pulls in React, a .tsx component or the shared
// browser runtime. A test that deliberately checks one of those passes
// {allow:['runtime']} or {allow:['react']} (for a store that only exposes a hook).
import {build} from 'esbuild';
import path from 'node:path';
const source=path.resolve(import.meta.dirname,'../../src');
const cache=new Map();
export function loadSource(name,{allow=[]}={}){
 const key=[name,...allow].join('+');
 if(!cache.has(key))cache.set(key,bundle(name,new Set(allow)));
 return cache.get(key);
}
async function bundle(name,allow){
 const result=await build({
  entryPoints:[path.join(source,`${name}.ts`)],bundle:true,write:false,metafile:true,
  platform:'node',format:'esm',logLevel:'silent',define:{'process.env.NODE_ENV':'"production"'},
  plugins:[{name:'runtime-alias',setup(b){b.onResolve({filter:/^@collector\/runtime$/},()=>({path:'@collector/runtime',namespace:'forbidden'}));b.onLoad({filter:/.*/,namespace:'forbidden'},()=>({contents:''}));}}],
 });
 const forbidden=Object.keys(result.metafile.inputs).filter(input=>
  input.startsWith('forbidden:')||input.endsWith('.tsx')
  ||(!allow.has('react')&&/(^|\/)node_modules\/(react|react-dom|scheduler)\//.test(input))
  ||(!allow.has('runtime')&&/(^|\/)src\/runtime\.ts$/.test(input)));
 if(forbidden.length)throw Error(`${name}.ts pulls in browser-only code, so it can't be unit tested in Node: ${forbidden.join(', ')}`);
 return import('data:text/javascript;base64,'+Buffer.from(result.outputFiles[0].text).toString('base64'));
}
