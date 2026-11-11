// Sort the review only. File indices still refer to the original package order.
const audioFormats=['flac','wav','mp3'];
const names=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
export function audioFormat(name:string){
 const suffix=name.slice(name.lastIndexOf('.')+1).toLowerCase();
 return name.includes('.')&&audioFormats.includes(suffix)?suffix:undefined;
}
export function compareImportFiles(a:string,b:string){
 const first=audioFormats.indexOf(audioFormat(a)??'');
 const second=audioFormats.indexOf(audioFormat(b)??'');
 return (first<0?audioFormats.length:first)-(second<0?audioFormats.length:second)||names.compare(a,b);
}
