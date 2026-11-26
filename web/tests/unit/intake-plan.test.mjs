import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {intakeBatches,intakeBatchLimits:limits,intakeIntent,needsDecision,isDigitalPackage,albumRoot}=await loadSource('intake-plan');
const sized=(count,files=1,bytes=1)=>Array.from({length:count},()=>({files,bytes}));
const measure=value=>value;
const sizes=batches=>batches.map(({start,end})=>end-start);

test('batches stop at exactly 64 packages, 10,000 files or 12 MiB',()=>{
 assert.deepEqual(sizes(intakeBatches(sized(129),0,measure)),[64,64,1]);
 assert.deepEqual(sizes(intakeBatches(sized(3,5000),0,measure)),[2,1],'10,000 files fit; one more does not');
 assert.deepEqual(sizes(intakeBatches(sized(3,1,limits.bytes/2),0,measure)),[2,1]);
});

test('a package over a limit on its own still goes, alone',()=>{
 const batches=intakeBatches([{files:1,bytes:1},{files:20000,bytes:1},{files:1,bytes:1}],0,measure);
 assert.deepEqual(batches,[{start:0,end:1},{start:1,end:2},{start:2,end:3}]);
});

test('a resumed review continues at the saved count',()=>{
 assert.deepEqual(intakeBatches(sized(70),64,measure),[{start:64,end:70}]);
 assert.deepEqual(intakeBatches(sized(64),64,measure),[]);
});

test('each choice builds the documented import intent',()=>{
 const opus={bitrate_kbps:128};
 assert.deepEqual(intakeIntent({automatic:true,preserveOnly:false,music:true,audioChoice:'flac'}),
  {automatic:true,prepare:false,analysis:'automatic',audio_preparation:{codec:'flac'},reviewed_opus_import:undefined});
 assert.deepEqual(intakeIntent({automatic:true,preserveOnly:true,music:true,audioChoice:'flac',opusDefault:opus}),
  {automatic:false,prepare:false,analysis:'preserve_only',audio_preparation:undefined,reviewed_opus_import:undefined},'preserve only must not identify or prepare audio');
 assert.deepEqual(intakeIntent({automatic:false,preserveOnly:false,music:true,audioChoice:'default',opusDefault:opus}).reviewed_opus_import,opus);
 assert.equal(intakeIntent({automatic:true,preserveOnly:false,music:false,audioChoice:'flac'}).audio_preparation,undefined,'only music collections prepare audio');
 assert.deepEqual(intakeIntent({automatic:true,preserveOnly:false,music:true,audioChoice:'none'}).audio_preparation,{codec:'none'});
});

test('packages that need a decision are shown; settled ones in a large batch are not',()=>{
 const settled={package:{album_tracks:null},destination:{destination:'separate'}};
 const plan={blockers:[],music_layout:{needs_attention:false}};
 assert.equal(needsDecision(settled,plan,{total:11,expanded:false}),false);
 assert.equal(needsDecision(settled,plan,{total:10,expanded:false}),true,'small batches show every package');
 assert.equal(needsDecision(settled,{...plan,blockers:['x']},{total:11,expanded:false}),true);
 assert.equal(needsDecision(settled,{...plan,music_layout:{needs_attention:true}},{total:11,expanded:false}),true);
 assert.equal(needsDecision({...settled,package:{album_tracks:[]}},plan,{total:11,expanded:false}),true,'an album with no tracks chosen needs a decision');
 assert.equal(needsDecision({...settled,destination:{destination:'album_group',copy_key:' '}},plan,{total:11,expanded:false}),true,'joining an album needs a copy name');
});

test('audio without a CUE or TOC is a digital purchase',()=>{
 assert.equal(isDigitalPackage(['01.flac','cover.jpg']),true);
 assert.equal(isDigitalPackage(['disc.bin','disc.cue','01.wav']),false);
 assert.equal(isDigitalPackage(['disc.toc','notes.txt']),false);
 assert.equal(isDigitalPackage(['flac']),false);
});

test('a folder is one album when audio sits in it, a format folder or a disc folder',()=>{
 assert.equal(albumRoot(['Album/01.flac']),true);
 assert.equal(albumRoot(['Album/FLAC/01.flac']),true);
 assert.equal(albumRoot(['Album/Disc 2/01.wav','Album/CD-1/01.wav']),true);
 assert.equal(albumRoot(['Shelf/Some Album/01.mp3']),false,'a folder of albums');
 assert.equal(albumRoot(['Album/cover.jpg']),false);
});
