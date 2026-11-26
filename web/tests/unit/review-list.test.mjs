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

const review=await loadSource('review-list');
const {reasonLevels}=await loadSource('review-reasons');
const scope=tab=>({collection:'c1',search:'blue',reason:'',system:'',tab});
const query=path=>Object.fromEntries(new URL(path,'https://collector.example').searchParams);

test('each Review tab sends the filters it claims',()=>{
 assert.equal(query(review.entryListPath(scope('needs'),null)).needs_attention,'true');
 const polish=query(review.entryListPath(scope('polish'),null));
 assert.equal(polish.level,'polish');assert.equal(polish.needs_attention,'false');
 const all=query(review.entryListPath(scope('all'),null));
 assert.equal(all.needs_attention,'false','All entries must not filter to entries needing attention');
 assert.equal(all.level,undefined,'All entries must not filter by level');
 const next=query(review.entryListPath({...scope('needs'),reason:'no_match',system:'snes'},'abc'));
 assert.deepEqual([next.collection_id,next.search,next.reason,next.setup_system,next.cursor],['c1','blue','no_match','snes','abc']);
});

test('"Select all matching" covers the same entries as the list',()=>{
 for(const tab of review.entryTabs){
  const listed=query(review.entryListPath({...scope(tab),reason:'no_match',system:'snes'},null));
  const chosen=review.matchingEntriesQuery({...scope(tab),reason:'no_match',system:'snes'});
  assert.equal(String(chosen.needs_attention),listed.needs_attention,tab);
  assert.equal(chosen.level??undefined,listed.level,tab);
  assert.deepEqual([chosen.collection_id,chosen.search,chosen.reason,chosen.setup_system],[listed.collection_id,listed.search,listed.reason,listed.setup_system],tab);
 }
});

test('import lists stay inside the chosen batch',()=>{
 const q=query(review.ingestListPath({collection:'c1',intake:'k1'},'attention',null));
 assert.deepEqual([q.collection_id,q.status,q.intake_id],['c1','attention','k1']);
});

test('the Reason filter never offers a reason the tab can\'t show',()=>{
 assert.ok(review.reasonFilterOptions('polish').every(reason=>reasonLevels[reason]==='polish'));
 assert.ok(review.reasonFilterOptions('needs').every(reason=>!['polish','setup'].includes(reasonLevels[reason])));
 assert.equal(review.reasonFilterOptions('all').length,Object.keys(reasonLevels).length);
});

test('a list refreshes quickly only while rows are still changing',()=>{
 const ready={summary_state:'ready',matching:null};
 assert.ok(review.entryPollDelay([ready])>review.entryPollDelay([{...ready,summary_state:'pending'}]));
 assert.ok(review.entryPollDelay([ready])>review.entryPollDelay([{...ready,matching:{state:'running'}}]));
 assert.equal(review.entryPollDelay([ready,{...ready,matching:{state:'completed'}}]),review.entryPollDelay([]));
});

test('an unknown tab opens Needs you',()=>{
 assert.equal(review.reviewTab('nonsense'),'needs');
 assert.equal(review.reviewTab('polish'),'polish');
 assert.equal(new Set(review.reviewTabs.map(tab=>review.reviewTabLabel(tab))).size,review.reviewTabs.length);
});

test('a failed import says whether its originals were saved',()=>{
 const failed={permissions:{can_resume_upload:false},phase:'published',outcome:'failed'};
 assert.notEqual(review.ingestStateLabel({...failed,capture_revision:3}),review.ingestStateLabel({...failed,capture_revision:null}));
 assert.notEqual(review.ingestStateLabel({...failed,permissions:{can_resume_upload:true}}),review.ingestStateLabel(failed),'an upload waiting for files must not read as failed');
});

test('"Next entry" is empty at the end of the list or for an entry not in it',()=>{
 const rows=[{id:'a'},{id:'b'}];
 assert.equal(review.nextEntry(rows,'a'),rows[1]);
 assert.equal(review.nextEntry(rows,'b'),undefined);
 assert.equal(review.nextEntry(rows,'z'),undefined);
});

test('in "all matching" mode every row is ticked except the exclusions',()=>{
 assert.equal(review.rowChecked({id:'a'},{all:true,exclusions:['b'],chosen:[]}),true);
 assert.equal(review.rowChecked({id:'b'},{all:true,exclusions:['b'],chosen:[{id:'b'}]}),false);
 assert.equal(review.rowChecked({id:'b'},{all:false,exclusions:[],chosen:[{id:'b'}]}),true);
});
