export function pushWorkLocation(work:string){
 const url=new URL(location.href);url.searchParams.set('work',work);
 // Browse depth measures every entry since the results, including nested work.
 // A direct link has no recorded return target and must keep its replace fallback.
 const depth=history.state?.depth||0;
 history.pushState({...history.state,depth:depth?depth+1:0},'',url);
}
