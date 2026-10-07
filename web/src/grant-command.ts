import {api} from '@collector/runtime';
import type {OperationAttempt} from './operation-scope';
import type {components} from './generated/api';
type Grant=components['schemas']['GrantView'];
export async function saveGrant(attempt:OperationAttempt,collection:string,username:string,role:'viewer'|'curator'|'owner'){
 const path=`/collections/${collection}/grants/${encodeURIComponent(username)}`;
 const current=await attempt.wait(api<Grant|null>(path,{signal:attempt.signal}));
 await attempt.wait(api(path,{method:'PUT',signal:attempt.signal,body:JSON.stringify({expected_revision:current?.revision||0,role})}));
}
