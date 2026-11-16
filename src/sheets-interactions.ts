import {addressOf,parseCellAddress,parseRange} from './sheets-workbench';
export type SheetFilter={column:string;query:string};
export type SheetSelection={sheet:number;anchor:string;focus:string};
export function matchingRows(cells:Record<string,string>,filter:SheetFilter):number[]{
 const column=filter.column.toUpperCase();
 parseCellAddress(column+'1');
 const needle=filter.query.toLocaleLowerCase();
 if(!needle.trim())return [];
 const result:number[]=[];
 for(let row=1;row<=10000;row++){
  const value=cells[column+row]??'';
  if(value.toLocaleLowerCase().includes(needle))result.push(row);
 }
 return result;
}
export function selectedRect(selection:SheetSelection):{top:number;left:number;bottom:number;right:number;range:string}{
 const a=parseCellAddress(selection.anchor),b=parseCellAddress(selection.focus);
 const top=Math.min(a.row,b.row),left=Math.min(a.col,b.col),bottom=Math.max(a.row,b.row),right=Math.max(a.col,b.col);
 return {top,left,bottom,right,range:parseRange(addressOf(top,left)+':'+addressOf(bottom,right))};
}
export function selectedCellCount(selection:SheetSelection):number{
 const r=selectedRect(selection);
 return (r.bottom-r.top+1)*(r.right-r.left+1);
}
export function isSelected(selection:SheetSelection|null,sheet:number,address:string):boolean{
 if(!selection||selection.sheet!==sheet)return false;
 const pos=parseCellAddress(address),r=selectedRect(selection);
 return pos.row>=r.top&&pos.row<=r.bottom&&pos.col>=r.left&&pos.col<=r.right;
}
export function rangeAsTsv(cells:Record<string,string>,selection:SheetSelection,limit=5000):string{
 const r=selectedRect(selection);
 if(selectedCellCount(selection)>limit)throw Error('Copy is limited to 5,000 cells');
 const out:string[]=[];
 for(let row=r.top;row<=r.bottom;row++){
  const values:string[]=[];
  for(let col=r.left;col<=r.right;col++)values.push((cells[addressOf(row,col)]??'').replace(/[\t\r\n]/g,' '));
  out.push(values.join('\t'));
 }
 return out.join('\n');
}
