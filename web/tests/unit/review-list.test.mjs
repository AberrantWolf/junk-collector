import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';

const {proposesGrouping,groupingAllowed,maxGroupedAlbums}=await loadSource('review-list');

test('grouping opens from the entry\'s own review reasons',()=>{
// The entry's own review reasons decide, whether or not a loaded list row exists.
assert.equal(proposesGrouping(['grouping_proposal'],undefined),true,'an entry opened from a link must still open grouping');
assert.equal(proposesGrouping(['missing_artwork','grouping_proposal'],[]),true,'a row from another page must not hide grouping');
assert.equal(proposesGrouping([],['grouping_proposal']),false,'a stale list row must not reopen a settled grouping');
assert.equal(proposesGrouping(undefined,['grouping_proposal']),true,'the list row stands in until the review loads');
assert.equal(proposesGrouping(undefined,undefined),false,'no information means no proposal');
});

test('the group button matches what grouping accepts',()=>{
const music=(count,collection=()=> 'c1')=>Array.from({length:count},(_,index)=>({module:'org.junk.music',collection_id:collection(index)}));
assert.equal(groupingAllowed(music(1)),false,'one entry cannot be grouped');
assert.equal(groupingAllowed(music(2)),true);
assert.equal(groupingAllowed(music(maxGroupedAlbums)),true,'the server limit itself is allowed');
assert.equal(groupingAllowed(music(maxGroupedAlbums+1)),false,'the button must match the server limit');
assert.equal(groupingAllowed(music(2,index=>'c'+index)),false,'entries from different collections cannot be grouped');
assert.equal(groupingAllowed([...music(1),{module:'org.junk.games',collection_id:'c1'}]),false,'only music entries can be grouped');
});
