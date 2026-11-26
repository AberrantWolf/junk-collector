import {build} from 'esbuild';
import {readFile,writeFile} from 'node:fs/promises';
import {sourceDigest} from './source-digest.mjs';
await build({entryPoints:['src/app.tsx'],bundle:true,minify:true,format:'esm',outfile:'dist/app.js',define:{'process.env.NODE_ENV':'"production"'}});
await build({entryPoints:['src/playback-sw.ts'],bundle:true,minify:true,format:'iife',outfile:'dist/playback-sw.js'});
// Management entrypoints use the same React instance exported by app.js.
for(const entry of ['curation','administration','security','work-history','collection-access','library-metadata','library-imports','library-preparation','library-activity','purchase-inbox']) await build({entryPoints:[`src/${entry}.tsx`],bundle:true,minify:true,format:'esm',outfile:`dist/${entry}.js`,plugins:[{name:'runtime',setup(b){b.onResolve({filter:/^@collector\/runtime$/},()=>({path:'/app.js',external:true}))}}],jsxFactory:'React.createElement',jsxFragment:'React.Fragment',jsx:'transform'});
const styles=['app','assets','imports','jobs','music-tracks','metadata-review','dialog'];
await writeFile('dist/app.css',(await Promise.all(styles.map(name=>readFile(`src/${name}.css`,'utf8')))).join('\n'));
// Lets the server build refuse (or redo) a build from outdated assets.
await writeFile('dist/source-digest',sourceDigest()+'\n');
