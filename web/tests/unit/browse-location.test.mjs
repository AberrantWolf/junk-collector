import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {parseLocation,browseUrl}=await loadSource('browse-location');
const at=url=>{const parsed=new URL(url,'https://collector.example');return parseLocation(parsed.pathname,parsed.search);};

test('a shared link opens the same view it was copied from',()=>{
 for(const url of [
  '/',
  '/?collection=c1&search=blue&sort=artist&filter=playable&item=i1',
  '/review?collection=c1&reason=no_match&system=snes&tab=polish',
  '/review?tab=all&intake=k1&ingest=g1&attention=all',
  '/imports?attach=i9',
  '/lists?list=l1',
  '/library-care/music-preparation?opus_bitrate_kbps=96&attention=all',
  '/purchase-inbox',
  '/activity',
 ])assert.equal(browseUrl(at(url)),url,url);
});

test('unknown values fall back to defaults instead of reaching the server',()=>{
 const view=at('/?sort=drop_table&filter=everything&opus_bitrate_kbps=999');
 assert.equal(view.sort,'title');assert.equal(view.filter,'all');assert.equal(view.preparationPreset,'flac');
 assert.equal(at('/nowhere').page,'library');
});

test('filters that belong to one page are not carried onto another',()=>{
 const review=at('/review?reason=no_match&system=snes&tab=polish&intake=k1');
 assert.equal(browseUrl({...review,page:'library'}),'/');
 const imports=at('/imports?attach=i9');
 assert.equal(browseUrl({...imports,page:'metadata'}),'/review');
});

test('the old Review address still opens Review',()=>{
 assert.equal(browseUrl(at('/library-care/metadata?reason=lookup_failed')),'/review?reason=lookup_failed');
});

test('"show all" on Review opens the All tab, but not on preparation',()=>{
 assert.equal(at('/review?attention=all').tab,'all');
 assert.equal(at('/library-care/music-preparation?attention=all').tab,'');
 assert.equal(at('/review').attention,true);
});
