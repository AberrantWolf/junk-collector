import {chromium as playwrightChromium,expect} from '@playwright/test';
import {accessSync,constants} from 'node:fs';
import {createHash,X509Certificate} from 'node:crypto';
import {deadline,onCleanup,cleanup} from './runtime.mjs';
export {expect};
// Exact disposable-fixture trust also covers service-worker script fetches.
export function fixtureCertificateArgs(pem){
 const key=new X509Certificate(pem).publicKey.export({type:'spki',format:'der'});
 return ['--ignore-certificate-errors-spki-list='+createHash('sha256').update(key).digest('base64')];
}
export function assertBrowserEnvironment({platform=process.platform,sandbox=process.env.CODEX_SANDBOX}={}){
 if(platform==='darwin'&&sandbox==='seatbelt')throw Error('Browser tests cannot launch inside the macOS Codex seatbelt sandbox (Chrome aborts during application registration). Run this test through the permitted terminal execution context. No browser was launched.');
}
// Fail before a test creates servers, profiles or expensive fixtures.
assertBrowserEnvironment();
export async function launchBrowser(options={}){
 assertBrowserEnvironment();
 const {mode=process.env.COLLECTOR_TEST_BROWSER||'chromium',...launch}=options;
 if(!['chromium','chrome'].includes(mode))throw Error('COLLECTOR_TEST_BROWSER must be chromium or chrome');
 const browserOptions={...launch,headless:true};
 if(process.env.CHROME_PATH)browserOptions.executablePath=process.env.CHROME_PATH;
 else if(mode==='chrome')browserOptions.channel='chrome';
 else {
  try{accessSync(playwrightChromium.executablePath(),constants.X_OK);}catch{throw Error('Test browser is missing. Run npm --prefix web exec -- playwright install chromium, then retry.');}
 }
 let server;
 try{server=await playwrightChromium.launchServer(browserOptions);}catch(error){throw Error(`Browser infrastructure failure (${mode}): ${error.message}`,{cause:error});}
 const child=server.process();
 const unregister=onCleanup(close);
 let browser,closing;
 async function close(){
  if(closing)return closing;
  closing=(async()=>{
   try{await deadline(server.close(),'browser shutdown',5000);}catch(error){
    try{await deadline(server.kill(),'browser reap',5000);}catch{child?.kill('SIGKILL');}
    throw error;
   }finally{unregister();}
  })();return closing;
 }
 try{browser=await playwrightChromium.connect(server.wsEndpoint());}catch(error){await close();throw error;}
 browser.close=close;
 browser.fixtureProcess=child;
 const context=browser.newContext.bind(browser);
 browser.newContext=async options=>{const result=await context(options);result.setDefaultTimeout(15000);result.setDefaultNavigationTimeout(30000);return result;};
 return browser;
}
export const chromium={launch:launchBrowser};
// Runs even if diagnostics in an old test's catch/finally throw.
process.once('beforeExit',()=>void cleanup().catch(error=>{console.error(error);process.exitCode=1;}));
