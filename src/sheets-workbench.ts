/** UI utilities for the bounded Sheets viewport and range inputs. */
export type GridViewport={row:number;col:number;rows:number;cols:number};
export const SHEET_MAX_ROW=10_000;
export const SHEET_MAX_COL=702;
export function colName(n:number):string{
 if(!Number.isInteger(n)||n<1||n>SHEET_MAX_COL)throw Error('Column outside supported grid');
 let s='';while(n){n--;s=String.fromCharCode(65+n%26)+s;n=Math.floor(n/26);}return s;
}
export function addressOf(row:number,col:number):string{
 if(!Number.isInteger(row)||row<1||row>SHEET_MAX_ROW)throw Error('Row outside supported grid');
 return colName(col)+row;
}
export function parseCellAddress(value:string):{row:number;col:number}{
 const matched=/^([A-Z]{1,3})([1-9][0-9]{0,4})$/.exec(value.trim().toUpperCase());
 if(!matched)throw Error('Use a cell address such as A1, C450 or ZZ99');
 const col=[...matched[1]].reduce((n,c)=>n*26+c.charCodeAt(0)-64,0);
 const row=Number(matched[2]);
 if(col>SHEET_MAX_COL||row>SHEET_MAX_ROW)throw Error('Address is outside supported grid');
 return {row,col};
}
export function viewportAround(address:string,current:GridViewport):GridViewport{
 const {row,col}=parseCellAddress(address);
 return {...current,row:Math.min(Math.max(1,row-Math.floor(current.rows/3)),SHEET_MAX_ROW-current.rows+1),
  col:Math.min(Math.max(1,col-Math.floor(current.cols/3)),SHEET_MAX_COL-current.cols+1)};
}
export function nextViewport(current:GridViewport,rowDelta:number,colDelta:number):GridViewport{
 return {...current,row:Math.min(Math.max(1,current.row+rowDelta),SHEET_MAX_ROW-current.rows+1),
  col:Math.min(Math.max(1,current.col+colDelta),SHEET_MAX_COL-current.cols+1)};
}
export function parseRange(text:string):string{
 const range=text.trim().toUpperCase();
 const parts=range.split(':');
 if(parts.length!==2)throw Error('Select a range such as A1:D40');
 const a=parseCellAddress(parts[0]),b=parseCellAddress(parts[1]);
 if(a.row>b.row||a.col>b.col)throw Error('Range must start at upper-left');
 return addressOf(a.row,a.col)+':'+addressOf(b.row,b.col);
}
