import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {acceptable,reasonLabels,reasonActions,reasonLevels}=await loadSource('review-reasons');

test('"Accept as is" never settles a problem that accepting cannot fix',()=>{
 for(const reason of ['lookup_failed','needs_main_file','no_catalog','grouping_proposal'])assert.equal(acceptable(reason),false,reason);
 assert.equal(acceptable('missing_artwork'),true);
 assert.equal(acceptable('no_match'),true);
});

test('every reason has a label, a first action and a level',()=>{
 const reasons=Object.keys(reasonLabels);
 assert.deepEqual(Object.keys(reasonActions).sort(),[...reasons].sort());
 assert.deepEqual(Object.keys(reasonLevels).sort(),[...reasons].sort());
 assert.equal(new Set(Object.values(reasonLabels)).size,reasons.length,'two reasons share a label, so a curator cannot tell them apart');
});
