import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {audioFormat,compareImportFiles,automaticTrackNumbers,freezeTrackOrder,orderGroup}=await loadSource('import-file-order');
const track=(file_name,extra={})=>({file_name,disc_position:null,display_position:null,...extra});

test('lossless audio sorts first, then names in natural number order',()=>{
 const files=['b.mp3','notes.txt','10 Ten.flac','2 Two.flac','cover.jpg','1 One.wav'];
 assert.deepEqual([...files].sort(compareImportFiles),['2 Two.flac','10 Ten.flac','1 One.wav','b.mp3','cover.jpg','notes.txt']);
 assert.equal(audioFormat('flac'),undefined,'a bare word without a dot is not a file type');
 assert.equal(audioFormat('Track.FLAC'),'flac');
});

test('automatic numbers skip positions the curator already chose, per disc',()=>{
 const files=['a.flac','b.flac','c.flac','d.flac'].map(name=>({name}));
 const tracks=[track('a.flac'),track('b.flac',{display_position:1}),track('c.flac'),track('d.flac',{disc_position:2})];
 assert.deepEqual([...automaticTrackNumbers(files,tracks)],[['a.flac',2],['c.flac',3],['d.flac',1]]);
});

test('freezing the order touches only discs the curator ordered',()=>{
 const files=['cd1/a.flac','cd1/b.flac','cd2/a.flac'].map(name=>({name}));
 const tracks=[track('cd1/a.flac',{disc_position:1}),track('cd1/b.flac',{disc_position:1}),track('cd2/a.flac',{disc_position:2})];
 assert.equal(freezeTrackOrder(files,tracks,[]),tracks,'no manual order leaves the tracks unchanged');
 // Moving one track on disc 1 fixes the rest of disc 1 in place, not disc 2.
 const moved=[{...tracks[0],display_position:2},tracks[1],tracks[2]];
 assert.deepEqual(freezeTrackOrder(files,moved,[]).map(t=>t.display_position),[2,1,null]);
 // Reordering the disc folders themselves fixes every disc below them.
 assert.deepEqual(freezeTrackOrder(files,tracks,[orderGroup('',null)]).map(t=>t.display_position),[1,2,1]);
});
