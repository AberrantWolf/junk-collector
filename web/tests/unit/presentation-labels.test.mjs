import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';

test('presentation labels preserve uncertainty and task meaning',async()=>{
const {presentationLabel}=await loadSource('presentation-labels');
const outcomes=['verified','incomplete_search','mismatched','no_data','ambiguous_offsets','unavailable','unsupported'];
assert.equal(new Set(outcomes.map(value=>presentationLabel(value))).size,outcomes.length,'distinct audio results must not collapse into the same label');
const unknown=presentationLabel('future_machine_state');
assert.ok(typeof unknown==='string'&&unknown.trim(),'unknown states need readable fallback text');
assert.notEqual(unknown,'future_machine_state','unknown machine tokens must not become UI labels');
for(const code of ['tentative','metadata_resolution','cancel_requested'])assert.notEqual(presentationLabel(code),unknown,`${code} lost its known-state label`);
assert.notEqual(presentationLabel('tentative'),presentationLabel('verified'),'a proposed identity must not be labelled verified');
assert.notEqual(unknown,presentationLabel('verified'),'an unknown result must not claim verification');
});
