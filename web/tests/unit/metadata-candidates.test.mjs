import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {activeCandidate,metadataSourceLabel,candidateList,identityKey,resumableLookup}=await loadSource('metadata-candidates');
const music='org.junk.music';
const candidate=(id,extra={})=>({identity:{provider:'org.junk.providers.musicbrainz',provider_record_id:id,provider_record_version:null},title:{value:`Album ${id}`},subtitle:null,selected:false,accepted_automatically:false,unmatched:false,current:false,strength:'name',...extra});
const options={module:music,showOthers:false,search:'',limit:10};

test('chosen, accepted and strong current matches always show; weak ones wait behind "show others"',()=>{
 const all=[candidate('a',{selected:true}),candidate('b',{accepted_automatically:true}),candidate('c',{current:true,strength:'disc_layout'}),candidate('d',{current:true,strength:'name'}),candidate('e')];
 const list=candidateList(all,options);
 assert.deepEqual(list.listed.map(c=>c.identity.provider_record_id),['a','b','c']);
 assert.equal(list.hidden,2);
 assert.equal(candidateList(all,{...options,showOthers:true}).hidden,0);
 assert.equal(candidateList(all,{...options,module:'org.junk.games'}).hidden,0,'only music narrows the list');
});

test('the pinned match goes first, then current matches, up to the limit',()=>{
 const all=[candidate('old',{selected:true}),candidate('new',{selected:true,current:true}),candidate('pin',{selected:true})];
 const shown=candidateList(all,{...options,limit:1,pinned:identityKey(all[2].identity)}).shown.map(c=>c.identity.provider_record_id);
 assert.deepEqual(shown,['pin','new'],'the pinned match must not count against the limit');
});

test('search narrows by title, subtitle or record ID',()=>{
 const all=[candidate('x1',{selected:true,title:{value:'Blue Album'}}),candidate('x2',{selected:true,subtitle:{value:'Deluxe BLUE'}}),candidate('needle',{selected:true}),candidate('x3',{selected:true})];
 assert.deepEqual(candidateList(all,{...options,search:' blue '}).matching.map(c=>c.identity.provider_record_id),['x1','x2']);
 assert.deepEqual(candidateList(all,{...options,search:'NEEDLE'}).matching.map(c=>c.identity.provider_record_id),['needle']);
});

test('a tentative identity is never labelled as the one in use',()=>{
 const match=candidate('a');
 const review={display:{resolution:'tentative',candidate:match.identity},candidates:[match]};
 assert.equal(activeCandidate(review),undefined);
 assert.equal(metadataSourceLabel(activeCandidate(review),music,false),metadataSourceLabel(undefined,music,false));
 assert.equal(activeCandidate({...review,display:{resolution:'selected',candidate:match.identity}}),match);
});

test('the status line tells imported tags, a provider match, manual edits and nothing apart',()=>{
 const tags=candidate('t',{identity:{provider:'org.junk.music.file-tags',provider_record_id:'t',provider_record_version:null}});
 const labels=[metadataSourceLabel(tags,music,false),metadataSourceLabel(candidate('m'),music,false),metadataSourceLabel(undefined,music,true),metadataSourceLabel(undefined,music,false)];
 assert.equal(new Set(labels).size,4);
});

test('a reload follows a running lookup, or offers the newest retryable one unless dismissed',()=>{
 const lookup=(id,state,retryable=false)=>({job:{id,state,retryable}});
 assert.equal(resumableLookup([lookup('new','failed',true),lookup('old','running')],new Set()).job.id,'old');
 assert.equal(resumableLookup([lookup('new','failed',true)],new Set(['new'])),undefined,'a dismissed failed lookup came back');
 assert.equal(resumableLookup([lookup('new','completed'),lookup('old','failed',true)],new Set()),undefined,'only the newest lookup is offered for retry');
});
