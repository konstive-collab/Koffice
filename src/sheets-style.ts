import {addressOf,parseCellAddress} from './sheets-workbench';
export type CellStyle={bold:boolean;italic:boolean;align:string;fill:string};
export type SheetFreeze={rows:number;cols:number};
export const EMPTY_STYLE:CellStyle={bold:false,italic:false,align:'',fill:''};
const fills=new Set(['','yellow','green','blue','red','gray']);
const aligns=new Set(['','left','center','right']);
export function validateStyle(style:CellStyle):CellStyle{
 if(!fills.has(style.fill)||!aligns.has(style.align))throw Error('Unsupported cell style');
 return {bold:!!style.bold,italic:!!style.italic,align:style.align,fill:style.fill};
}
export function styleClass(style:CellStyle|undefined):string{
 if(!style)return '';
 const s=validateStyle(style);
 return [s.bold?'s-bold':'',s.italic?'s-italic':'',s.align?'s-align-'+s.align:'',s.fill?'s-fill-'+s.fill:''].filter(Boolean).join(' ');
}
export function styleUpdates(range:string,style:CellStyle):Record<string,CellStyle>{
 const [a,b]=range.split(':');
 if(!a||!b)throw Error('Expected range');
 const top=parseCellAddress(a),bottom=parseCellAddress(b);
 const count=(bottom.row-top.row+1)*(bottom.col-top.col+1);
 if(count<1||count>5000)throw Error('Select 1–5,000 cells');
 const result:Record<string,CellStyle>={};
 for(let row=top.row;row<=bottom.row;row++)for(let col=top.col;col<=bottom.col;col++)result[addressOf(row,col)]=validateStyle(style);
 return result;
}
export function gridRows(scrollStart:number,size:number,freeze:number,limit=10000):number[]{
 const pinned=Array.from({length:freeze},(_,i)=>i+1);
 const scrolling=Array.from({length:size-freeze},(_,i)=>Math.min(limit,Math.max(freeze+1,scrollStart)+i));
 return [...pinned,...scrolling.filter((row,i)=>row<=limit&&(i===0||row!==scrolling[i-1]))];
}
