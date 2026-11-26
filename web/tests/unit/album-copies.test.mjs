import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const copies=await loadSource('album-copies');
const {albumCopies,currentCopy,copySelectionChanges,formatChoices,formatSelectionChanges,mergeTrackOverrides,preferredAvailable,groupDiscs}=copies;

// One track row. A CD rip shows up only as its saved FLAC (the disc image itself
// isn't a track file); a purchase's files are originals that play as they are.
const ready=(media_type='audio/flac')=>({status:'available',media_type,length:1,representation_id:'r',file_index:0});
let next=0;
function row(source,{position=++next,disc=1,profile=null,codec='flac',selection='included',capture='cap-'+source,state=ready()}={}){
 return {capture_id:capture,source_representation_id:source,track_position:position,disc_position:disc,display_position:null,profile,album_selection:selection,audio_format:codec?{codec}:null,source_label:source+' files',state,can_stream_original:false};
}
const rip=(source,disc,{selection}={})=>[row(source,{position:1,disc,profile:{codec:'flac'},selection})];
const purchase=(source,disc,{selection,positions=[1,2],codec='flac'}={})=>positions.map(position=>row(source,{position,disc,codec,selection}));
const label=id=>id;

test('a CD set plus one purchase offers two copies, and the album plays from one',()=>{
 const rows=[...rip('cd1',1),...rip('cd2',2),...purchase('shop',1,{selection:'excluded'}),...purchase('shop',2,{selection:'excluded',positions:[3]})];
 const found=albumCopies(rows,label);
 assert.deepEqual(found.map(copy=>copy.key),['discs','shop']);
 assert.match(found[0].label,/2 discs/);
 assert.match(found[1].label,/shop/);
 assert.equal(currentCopy(found),'discs');
});

test('purchases that hold different discs are one copy; two of the same disc are two',()=>{
 assert.deepEqual(albumCopies([...purchase('a',1),...purchase('b',2)],label),[],'disc 1 and disc 2 from separate purchases must not be offered as a choice');
 assert.equal(albumCopies([...purchase('a',1),...purchase('b',1)],label).length,2);
 assert.equal(currentCopy(albumCopies([...purchase('a',1),...purchase('b',1)],label)),'','both in use means no single current copy');
});

test('choosing a copy leaves out every track of the others and keeps their edits',()=>{
 const rows=[...purchase('a',1),...purchase('b',1)];
 const changes=copySelectionChanges(albumCopies(rows,label),'a');
 assert.deepEqual(changes,[{source:'b',position:1,selected:false},{source:'b',position:2,selected:false}]);
 const existing=[
  {source_representation_id:'b',track_position:1,title:'Renamed',disc_position:null,display_position:4,album_selected:null},
  {source_representation_id:'other',track_position:1,title:'Untouched',disc_position:null,display_position:null,album_selected:null},
 ];
 const saved=mergeTrackOverrides(existing,changes);
 assert.deepEqual(saved.find(value=>value.source_representation_id==='other'),existing[1],'an override for another source changed');
 assert.deepEqual(saved.filter(value=>value.source_representation_id==='b'),[
  {source_representation_id:'b',track_position:1,title:'Renamed',disc_position:null,display_position:4,album_selected:false},
  {source_representation_id:'b',track_position:2,title:null,disc_position:null,display_position:null,album_selected:false},
 ]);
});

test('the empty choice brings back only copies that were left out whole',()=>{
 const rows=[...purchase('a',1),...purchase('b',1,{selection:'excluded'}),...purchase('c',1).map((r,i)=>i?{...r,album_selection:'excluded'}:r)];
 const changes=copySelectionChanges(albumCopies(rows,label),'');
 assert.deepEqual(changes.map(change=>change.source),['b','b'],'a copy still partly in use was reset');
 assert.ok(changes.every(change=>change.selected===null));
});

test('an override with nothing left in it is dropped instead of saved empty',()=>{
 const existing=[{source_representation_id:'b',track_position:1,title:null,disc_position:null,display_position:null,album_selected:false}];
 assert.deepEqual(mergeTrackOverrides(existing,[{source:'b',position:1,selected:null}]),[]);
});

test('a format choice is offered only for an original in use with more than one codec',()=>{
 const mixed=[...purchase('shop',1,{positions:[1,2],codec:'mp3',selection:'excluded'}),...purchase('shop',1,{positions:[3,4],codec:'wav'})];
 assert.deepEqual(formatChoices(mixed),[{id:'shop',label:'shop files',codecs:['mp3','wav'],current:'wav'}]);
 assert.deepEqual(formatChoices(mixed.map(r=>({...r,album_selection:'excluded'}))),[],'choosing a format would quietly bring back a copy the album left out');
 assert.deepEqual(formatChoices(purchase('one',1)),[],'a single codec has nothing to choose');
});

test('"Automatic" format clears the selection but keeps other per-track edits',()=>{
 const mixed=[...purchase('shop',1,{positions:[1],codec:'mp3'}),...purchase('shop',1,{positions:[2],codec:'wav'})];
 assert.deepEqual(formatSelectionChanges(mixed,'shop','wav').map(change=>change.selected),[false,true]);
 const existing=[{source_representation_id:'shop',track_position:2,title:'Kept',disc_position:null,display_position:null,album_selected:true}];
 assert.deepEqual(mergeTrackOverrides(existing,formatSelectionChanges(mixed,'shop','')),[{...existing[0],album_selected:null}]);
});

test('playback prefers an original, then FLAC, and skips formats the player can\'t decode',()=>{
 const original=row('a',{state:ready('audio/wav')}),flac=row('a',{profile:{codec:'flac'}}),opus=row('a',{profile:{codec:'opus',bitrate_kbps:128},state:ready('audio/ogg')});
 assert.equal(preferredAvailable([opus,flac,original],()=>true),original);
 assert.equal(preferredAvailable([opus,flac,original],type=>type!=='audio/wav'),flac);
 assert.equal(preferredAvailable([opus,flac],type=>type==='audio/ogg'),opus);
 assert.equal(preferredAvailable([opus],()=>false),undefined);
});

test('tracks group by disc in album order, with every format of a track together',()=>{
 const rows=[row('b',{position:2,capture:'x'}),row('a',{position:2,capture:'y'}),row('a',{position:1,capture:'y'}),row('a',{position:1,capture:'y',profile:{codec:'flac'}})];
 const grouped=groupDiscs(rows,key=>key==='y/a'?0:1);
 assert.deepEqual(grouped.map(([key])=>key),['y/a','x/b']);
 assert.deepEqual(grouped[0][1].map(variants=>variants.length),[2,1]);
});
