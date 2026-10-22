import {build} from '../../../../web/node_modules/esbuild/lib/main.js';
import path from 'node:path';
const directory=import.meta.dirname;
await build({entryPoints:[path.join(directory,'app.tsx')],outfile:process.argv[2]||path.join(directory,'app.js'),bundle:true,format:'esm',platform:'browser',jsx:'automatic',nodePaths:[path.resolve(directory,'../../../../web/node_modules')],define:{'process.env.NODE_ENV':'"production"'},minify:true});
