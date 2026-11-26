import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const values=new Map();
globalThis.localStorage={getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)};
const model=await loadSource('playback-preference',{allow:['react']});

test('each account keeps its own playback format',()=>{
 model.setPlaybackPreferenceUser('alice');model.setPlaybackPreference({format:'low_bandwidth',bitrate:64});
 model.setPlaybackPreferenceUser('bob');assert.equal(model.getPlaybackPreference().format,'high_bandwidth','bob inherited alice\'s choice');
 model.setPlaybackPreferenceUser('alice');assert.deepEqual(model.getPlaybackPreference(),{format:'low_bandwidth',bitrate:64});
});

test('signed out, invalid or tampered choices are ignored',()=>{
 model.setPlaybackPreferenceUser(undefined);model.setPlaybackPreference({format:'original',bitrate:128});
 assert.equal(model.getPlaybackPreference().format,'high_bandwidth');
 model.setPlaybackPreferenceUser('carol');
 for(const bad of [{format:'original',bitrate:5},{format:'original',bitrate:257},{format:'original',bitrate:96.5},{format:'lossless',bitrate:128}]){
  model.setPlaybackPreference(bad);assert.equal(model.getPlaybackPreference().format,'high_bandwidth',JSON.stringify(bad));
 }
 values.set('collector.playback-format.v2.dave',JSON.stringify({format:'original',bitrate:9999}));
 model.setPlaybackPreferenceUser('dave');assert.equal(model.getPlaybackPreference().format,'high_bandwidth');
});
