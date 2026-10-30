import {build} from 'esbuild';
import {readFile,writeFile} from 'node:fs/promises';
await build({entryPoints:['src/app.tsx'],bundle:true,minify:true,format:'esm',outfile:'dist/app.js',define:{'process.env.NODE_ENV':'"production"'}});
await build({entryPoints:['src/playback-sw.ts'],bundle:true,minify:true,format:'iife',outfile:'dist/playback-sw.js'});
// Management entrypoints use the same React instance exported by app.js.
for(const entry of ['curation','administration','security','work-history','collection-access','library-metadata','library-imports','library-preparation','library-activity','purchase-inbox']) await build({entryPoints:[`src/${entry}.tsx`],bundle:true,minify:true,format:'esm',outfile:`dist/${entry}.js`,plugins:[{name:'runtime',setup(b){b.onResolve({filter:/^@collector\/runtime$/},()=>({path:'/app.js',external:true}))}}],jsxFactory:'React.createElement',jsxFragment:'React.Fragment',jsx:'transform'});
await writeFile('dist/app.css',`${await readFile('src/app.css','utf8')}\n${await readFile('src/assets.css','utf8')}\n${await readFile('src/imports.css','utf8')}\n${await readFile('src/jobs.css','utf8')}\n${await readFile('src/music-tracks.css','utf8')}\n${await readFile('src/metadata-review.css','utf8')}\n${await readFile('src/dialog.css','utf8')}`);
