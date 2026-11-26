// Which requests removing or deleting an entry sends, and what the dialog says
// while and after it does. The dialogs in remove-entry.tsx hold the state and
// send the requests; the decisions live here so they can be checked in Node.
import type {components} from './generated/api';

type Item=components['schemas']['PresentedItem'];
export type RemovalChoice='hide'|'files'|'everything';
// remove: hide the entry (it can be restored). purge: delete a removed entry
// for good. purge-files: delete only the entry's files; the entry stays.
export type RemovalStep='remove'|'purge'|'purge-files';
type Status={alreadyRemoved:boolean;removedHere:boolean};

// An already removed entry can only be deleted entirely.
export function initialRemovalChoice(item:Pick<Item,'deleted'>):RemovalChoice{return item.deleted?'everything':'hide';}

// Deleting only the files needs files, and makes no sense once this dialog has
// removed the entry on the way to deleting it.
export function removalChoiceDisabled(item:Pick<Item,'captures'>,choice:RemovalChoice,removedHere:boolean):boolean{
 return choice==='files'&&(item.captures.length===0||removedHere);
}

// The requests a confirmation sends, in order. A retry after a failed deletion
// doesn't remove the entry a second time.
export function removalSteps(choice:RemovalChoice,{alreadyRemoved,removedHere}:Status):RemovalStep[]{
 if(choice==='files')return ['purge-files'];
 const remove:RemovalStep[]=alreadyRemoved||removedHere?[]:['remove'];
 return choice==='hide'?remove:[...remove,'purge'];
}

export function removalButtonLabel(choice:RemovalChoice,alreadyRemoved:boolean,busy:boolean):string{
 if(busy)return choice==='hide'?'Removing…':'Deleting…';
 return choice==='hide'?'Remove entry':choice==='files'?'Delete files permanently':alreadyRemoved?'Delete permanently':'Remove and delete permanently';
}

// Says the entry can still be restored only when this dialog removed it and the
// deletion after that failed; otherwise the message stands alone.
export function removalFailureMessage(message:string,removedByThisDialog:boolean):string{
 return removedByThisDialog?`${message} The entry is removed and can still be restored.`:message;
}

export function originalBytes(item:Pick<Item,'captures'>,captures?:readonly string[]):number{
 return item.captures.filter(capture=>!captures||captures.includes(capture.id)).flatMap(capture=>capture.representations).flatMap(representation=>representation.files).reduce((total,file)=>total+file.bytes,0);
}

// The physical copy record an original came from, and whether another original
// of the same copy is still here. Deleting the record is offered only when it
// isn't shared, so the other discs keep their copy.
export function copyRecordChoice(item:Pick<Item,'captures'|'specimens'>,captureId:string){
 const capture=item.captures.find(value=>value.id===captureId);
 const specimen=capture?.association.specimen_id;
 const copy=specimen?item.specimens.find(value=>value.id===specimen):undefined;
 const shared=!!specimen&&item.captures.some(value=>value.id!==captureId&&value.association.specimen_id===specimen);
 return {copy,shared,offerDelete:!!copy&&!shared};
}

// Work on the entry stops or finishes before files can go, and the server
// answers 409 until it has. Retry only that answer, reading the entry again
// between attempts so the next request carries its current revision.
export async function withConflictRetry<T>(request:()=>Promise<T>,refresh:()=>Promise<void>,{attempts=15,wait=(ms:number)=>new Promise<void>(resolve=>setTimeout(resolve,ms)),isConflict}:{attempts?:number;wait?:(ms:number)=>Promise<void>;isConflict:(failure:unknown)=>boolean}):Promise<T>{
 for(let attempt=0;;attempt++){
  try{return await request();}
  catch(failure){
   if(!isConflict(failure)||attempt>=attempts)throw failure;
   await wait(1000);
   await refresh();
  }
 }
}
