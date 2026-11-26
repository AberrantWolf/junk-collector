import {test} from 'node:test';
import assert from 'node:assert/strict';
import {loadSource} from '../support/load-source.mjs';
const {initialRemovalChoice,removalChoiceDisabled,removalSteps,removalButtonLabel,removalFailureMessage,originalBytes,copyRecordChoice,withConflictRetry}=await loadSource('entry-removal');
const fresh={alreadyRemoved:false,removedHere:false};
const capture=(id,specimen,bytes=[10])=>({id,association:{specimen_id:specimen},representations:[{files:bytes.map(b=>({bytes:b}))}]});

test('an already removed entry offers only deleting everything, and sends only the purge',()=>{
 assert.equal(initialRemovalChoice({deleted:true}),'everything');
 assert.deepEqual(removalSteps('everything',{alreadyRemoved:true,removedHere:false}),['purge']);
 assert.equal(initialRemovalChoice({deleted:false}),'hide','the default must be the choice that can be undone');
});

test('hiding sends only the remove, which can be undone',()=>{
 assert.deepEqual(removalSteps('hide',fresh),['remove']);
});

test('deleting everything removes first, and a retry after a failed purge skips the remove',()=>{
 assert.deepEqual(removalSteps('everything',fresh),['remove','purge']);
 assert.deepEqual(removalSteps('everything',{alreadyRemoved:false,removedHere:true}),['purge']);
 assert.deepEqual(removalSteps('hide',{alreadyRemoved:false,removedHere:true}),[],'switching to hide after a failed purge must not remove twice');
});

test('deleting only the files never removes the entry',()=>{
 assert.deepEqual(removalSteps('files',fresh),['purge-files']);
 assert.equal(removalChoiceDisabled({captures:[]},'files',false),true,'nothing to delete');
 assert.equal(removalChoiceDisabled({captures:[capture('a')]},'files',true),true,'the entry is already removed');
 assert.equal(removalChoiceDisabled({captures:[capture('a')]},'files',false),false);
});

test('the button says when the action is permanent',()=>{
 assert.notEqual(removalButtonLabel('hide',false,false),removalButtonLabel('everything',false,false));
 assert.notEqual(removalButtonLabel('everything',false,false),removalButtonLabel('everything',true,false),'removing first must read differently from deleting a removed entry');
});

test('after a failure, the entry is said to be restorable only if this dialog removed it',()=>{
 assert.equal(removalFailureMessage('Busy.',false),'Busy.');
 assert.notEqual(removalFailureMessage('Busy.',true),'Busy.');
});

test('the size counts only the chosen originals',()=>{
 const item={captures:[capture('a','s1',[5,6]),capture('b','s1',[100])]};
 assert.equal(originalBytes(item),111);
 assert.equal(originalBytes(item,['a']),11);
});

test('deleting a copy record is offered only when no other original shares the copy',()=>{
 const specimens=[{id:'s1',label:'Box set'},{id:'s2',label:'Single'}];
 const item={captures:[capture('a','s1'),capture('b','s1'),capture('c','s2'),capture('d',null)],specimens};
 assert.deepEqual(copyRecordChoice(item,'a'),{copy:specimens[0],shared:true,offerDelete:false});
 assert.deepEqual(copyRecordChoice(item,'c'),{copy:specimens[1],shared:false,offerDelete:true});
 assert.equal(copyRecordChoice(item,'d').offerDelete,false);
});

test('only conflict answers are retried, with a refresh between, up to the limit',async()=>{
 const conflict=Object.assign(Error('conflict'),{status:409});
 const isConflict=failure=>failure?.status===409;
 const wait=async()=>{};
 let calls=0,refreshes=0;
 assert.equal(await withConflictRetry(async()=>{if(++calls<3)throw conflict;return 'done';},async()=>{refreshes++;},{isConflict,wait}),'done');
 assert.deepEqual([calls,refreshes],[3,2]);
 calls=0;await assert.rejects(withConflictRetry(async()=>{calls++;throw Error('gone');},async()=>{},{isConflict,wait}),/gone/);
 assert.equal(calls,1,'a non-conflict failure was retried');
 calls=0;await assert.rejects(withConflictRetry(async()=>{calls++;throw conflict;},async()=>{},{isConflict,wait,attempts:4}),/conflict/);
 assert.equal(calls,5,'the retry limit was not respected');
});
