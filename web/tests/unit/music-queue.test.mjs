// Personal queue persistence/isolation rules, independent of media decoding.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
import {randomUUID} from 'node:crypto';

test('personal queue isolation, ordering, storage limits and sign-out',async()=>{
const model=await loadSource('music-queue',{allow:['react']});
const values=new Map();globalThis.localStorage={getItem:key=>values.get(key)||null,setItem:(key,value)=>values.set(key,value)};
const key=user=>'collector.music-queue.v1.'+user;
const track=title=>({itemId:randomUUID(),collectionId:randomUUID(),album:'An album',title,position:1,representationId:randomUUID(),fileIndex:0,mediaType:'audio/flac'});
const first=track('First'),second=track('Second');
model.setMusicQueueUser('alice');model.addMusicTracks([first,second]);let alice=JSON.parse(values.get(key('alice')));const current=alice.current;
model.reorderMusicTrack(current,1);alice=JSON.parse(values.get(key('alice')));assert.equal(alice.current,current);assert.equal(alice.tracks[1].id,current,'reorder changed the selected immutable track');
model.setMusicQueueUser('bob');model.addMusicTracks([track('Bob only')]);assert.equal(JSON.parse(values.get(key('bob'))).tracks.length,1,'another user inherited the queue');
model.setMusicQueueUser('alice');model.removeMusicTrack(current);alice=JSON.parse(values.get(key('alice')));assert.equal(alice.tracks.length,1);assert.equal(alice.tracks[0].title,'Second');assert.equal(alice.current,alice.tracks[0].id);
model.setMusicQueueUser();assert.equal(model.addMusicTracks([first]),0,'signed-out activity mutated a personal queue');assert.equal(JSON.parse(values.get(key('alice'))).tracks.length,1);
values.set(key('malformed'),JSON.stringify({tracks:[{...first,id:randomUUID(),representationId:'../../foreign'}],current:null}));model.setMusicQueueUser('malformed');model.addMusicTracks([second]);assert.equal(JSON.parse(values.get(key('malformed'))).tracks.length,1,'invalid stored coordinates were trusted');
assert.equal(model.addMusicTracks(Array.from({length:5700},()=>first)),4999);assert.equal(model.addMusicTracks([second]),0);model.clearMusicQueue();assert.deepEqual(JSON.parse(values.get(key('malformed'))),{tracks:[],current:null});
globalThis.localStorage.setItem=()=>{throw new Error('Storage disabled');};assert.equal(model.addMusicTracks([first]),1,'storage failure prevented in-memory queue use');

model.clearMusicQueue();assert.equal(model.startMusicTracks([first,second],true),true);assert.equal(model.addMusicTracks([]),0);
assert.equal(model.startMusicTracks(Array.from({length:5001},()=>first)),false,'oversized playlist replaced the queue');
globalThis.localStorage.setItem=(k,v)=>values.set(k,v);model.setMusicQueueUser('alice');assert.equal(model.startMusicTracks([first,second]),true);const played=JSON.parse(values.get(key('alice')));assert.equal(played.tracks[0].title,'First');assert.equal(played.playRequest,played.current);model.setMusicQueueUser('bob');model.setMusicQueueUser('alice');model.addMusicTracks([]);assert.equal(JSON.parse(values.get(key('alice'))).playRequest,undefined,'reload restored autoplay');
});
