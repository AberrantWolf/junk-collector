// A fingerprint of everything web/dist is built from. build.mjs writes it to
// dist/source-digest, and the server's build script recomputes it (see
// crates/collector-server/build.rs), so a server is never built with web assets
// older than web/src. Keep the two implementations identical: FNV-1a 64 over each
// file's path, a zero byte, its contents and another zero byte, in path order.
import {readdirSync,readFileSync} from 'node:fs';
import path from 'node:path';
const web=import.meta.dirname;
const extensions=new Set(['.ts','.tsx','.css','.js','.mjs','.json','.html','.svg']);
export function sourceFiles(){
 const files=['build.mjs','package-lock.json'];
 const walk=directory=>{for(const entry of readdirSync(path.join(web,directory),{withFileTypes:true})){
  if(entry.name.startsWith('.'))continue;
  const relative=directory+'/'+entry.name;
  if(entry.isDirectory())walk(relative);else if(extensions.has(path.extname(entry.name)))files.push(relative);
 }};
 walk('src');
 return files.sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b)));
}
export function sourceDigest(){
 let hash=0xcbf29ce484222325n;const prime=0x100000001b3n,mask=0xffffffffffffffffn;
 const add=bytes=>{for(const byte of bytes)hash=((hash^BigInt(byte))*prime)&mask;};
 for(const file of sourceFiles()){add(Buffer.from(file));add([0]);add(readFileSync(path.join(web,file)));add([0]);}
 return hash.toString(16).padStart(16,'0');
}
