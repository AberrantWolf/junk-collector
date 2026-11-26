import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {resultLabel}=await loadSource('check-results');
const unit=algorithm=>({digest:{algorithm}});
const catalog=(match,units)=>({result:{catalog:match==='unique_complete_match'?{match,candidate:{matched_units:units}}:{match,candidates:[{matched_units:units}]}}});

test('a partial or ambiguous catalog match never reads like a full one',()=>{
 const full=resultLabel(catalog('unique_complete_match',[unit('sha256')]));
 for(const kind of ['ambiguous_complete_matches','partial_identification','no_match','inconclusive'])
  assert.notEqual(resultLabel(catalog(kind,[unit('sha256')])),full,kind);
 assert.notEqual(resultLabel(catalog('ambiguous_complete_matches',[])),resultLabel(catalog('partial_identification',[])));
});

test('a CRC32-only match never reads as cryptographic identity',()=>{
 const crc=resultLabel(catalog('unique_complete_match',[unit('crc32'),unit('crc32')]));
 const strong=resultLabel(catalog('unique_complete_match',[unit('sha1')]));
 assert.notEqual(crc,strong);
 assert.match(crc,/not cryptographic/);
 assert.match(resultLabel(catalog('partial_identification',[unit('crc32')])),/not cryptographic/,'the CRC32 caveat must survive a partial match too');
});

test('integrity, structure and verification results stay separate claims',()=>{
 const labels=[{result:'hashes'},{result:'track_hashes'},{result:'structure'},{result:'layout',tracks:[]},{result:'cd_audio',audio:{tracks:[]}},{result:'audio_verification',verification:{status:'verified'}}].map(result=>resultLabel({result}));
 assert.equal(new Set(labels).size,labels.length);
 assert.ok(labels.every(label=>!/catalog match/i.test(label)),'a check that compared no catalog claimed a catalog match');
});

test('an unknown result falls back to neutral text',()=>{
 const unknown=resultLabel({result:{result:'future_check'}});
 assert.equal(resultLabel({result:null}),unknown);
 assert.doesNotMatch(unknown,/match|verified|identity/i);
});
