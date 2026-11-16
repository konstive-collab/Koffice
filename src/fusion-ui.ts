/** Koffice Fusion UI: an editable, self-contained Sheets grid hosted inside Writer. */
export type EmbeddedSheet={kind:'sheet';id:string;label:string;workbook:{sheets:{cells:Record<string,string>}[];revision:number}};
const esc=(s:string)=>s.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const address=(row:number,mutCol:number)=>{let col='';while(mutCol>0){mutCol--;col=String.fromCharCode(65+mutCol%26)+col;mutCol=Math.floor(mutCol/26);}return col+row;};
const column=(n:number)=>address(1,n).replace(/\d+$/,'');
export class FusionUI{
 private root:HTMLDivElement;
 private objectId:string|null=null;
 private startRow=1;
 private startCol=1;
 constructor(
  private getObjects:()=>EmbeddedSheet[],
  private saveCells:(id:string,updates:Record<string,string>)=>Promise<void>,
  private remove:(id:string)=>Promise<void>,
  private extract:(id:string)=>Promise<void>,
  private notify:(message:string)=>void
 ){
  this.root=document.createElement('div');this.root.className='fusion-overlay';this.root.hidden=true;
  document.body.appendChild(this.root);
  this.root.addEventListener('keydown',e=>{if(e.key==='Escape'){e.stopPropagation();this.close();}});
 }
 private current(){return this.getObjects().find(x=>x.id===this.objectId);}
 preview():string{
  return this.getObjects().map(o=>{
   const cells=o.workbook.sheets[0].cells;
   const cols=Array.from({length:6},(_,i)=>column(i+1));
   const grid=Array.from({length:5},(_,r)=>'<tr>'+cols.map(c=>'<td>'+esc(cells[c+(r+1)]??'')+'</td>').join('')+'</tr>').join('');
   return '<section class="fusion-embed"><div class="fusion-embed-head"><strong>'+esc(o.label)+'</strong><span>'+Object.keys(cells).length+' cells · Sheets engine</span><button data-fusion-edit="'+esc(o.id)+'">Edit</button><button data-fusion-extract="'+esc(o.id)+'">To Sheets</button><button data-fusion-remove="'+esc(o.id)+'">Remove</button></div><div class="fusion-preview"><table><thead><tr>'+cols.map(c=>'<th>'+c+'</th>').join('')+'</tr></thead><tbody>'+grid+'</tbody></table></div></section>';
  }).join('');
 }
 /** Static printable view with complete cell content. Large sparse sheets use address/value listings. */
 exportHtml():string{
  return this.getObjects().map(obj=>{
   const cells=obj.workbook.sheets[0].cells;
   const entries=Object.entries(cells);
   const maxRow=entries.reduce((n,[a])=>Math.max(n,Number(a.match(/\d+$/)?.[0]??0)),0);
   const maxCol=entries.reduce((n,[a])=>Math.max(n,[...(a.match(/^[A-Z]+/)?.[0]??'A')].reduce((v,c)=>v*26+c.charCodeAt(0)-64,0)),0);
   let table='';
   if(maxRow*maxCol<=10000){
    const cols=Array.from({length:maxCol||1},(_,i)=>column(i+1));
    const header='<thead><tr><th></th>'+cols.map(x=>'<th>'+x+'</th>').join('')+'</tr></thead>';
    const rows=Array.from({length:maxRow||1},(_,i)=>'<tr><th>'+String(i+1)+'</th>'+cols.map(c=>'<td>'+esc(cells[c+(i+1)]??'')+'</td>').join('')+'</tr>').join('');
    table=header+'<tbody>'+rows+'</tbody>';
   }else{
    table='<thead><tr><th>Cell address</th><th>Value / formula</th></tr></thead><tbody>'+entries.map(([a,v])=>'<tr><th>'+esc(a)+'</th><td>'+esc(v)+'</td></tr>').join('')+'</tbody>';
   }
   return '<section style="page-break-inside:avoid;margin:2em 0"><h2>'+esc(obj.label)+'</h2><p>Embedded Koffice Sheets table (formula expressions shown as stored)</p><div style="overflow:auto"><table style="border-collapse:collapse;font:12px sans-serif">'+table+'</table></div></section>';
  }).join('');
 }
 wirePreview(){
  document.querySelectorAll<HTMLButtonElement>('[data-fusion-edit]').forEach(b=>b.onclick=()=>this.open(b.dataset.fusionEdit!));
  document.querySelectorAll<HTMLButtonElement>('[data-fusion-extract]').forEach(b=>b.onclick=()=>{
   void this.extract(b.dataset.fusionExtract!).catch(e=>this.notify(String(e)));
  });
  document.querySelectorAll<HTMLButtonElement>('[data-fusion-remove]').forEach(b=>b.onclick=()=>{
   if(window.confirm('Remove this embedded table?'))void this.remove(b.dataset.fusionRemove!).catch(e=>this.notify(String(e)));
  });
 }
 open(id:string){
  if(!this.getObjects().some(x=>x.id===id)){this.notify('Embedded table not found');return;}
  this.objectId=id;this.startRow=1;this.startCol=1;this.draw();
 }
 close(){this.objectId=null;this.root.hidden=true;this.root.replaceChildren();}
 refresh(){if(this.objectId)this.draw();}
 private async commit(updates:Record<string,string>){
  if(!this.objectId)return;
  try{await this.saveCells(this.objectId,updates);this.draw();}
  catch(e){this.notify(String(e));this.draw();}
 }
 private draw(){
  const obj=this.current();if(!obj){this.close();return;}
  const cols=Array.from({length:12},(_,i)=>column(this.startCol+i));
  const cells=obj.workbook.sheets[0].cells;
  const grid=Array.from({length:20},(_,r)=>{
   const row=this.startRow+r;if(row>10000)return '';
   return '<tr><th>'+row+'</th>'+cols.map(c=>{const a=c+row;return '<td><input data-fusion-cell="'+a+'" aria-label="'+a+'" spellcheck="false" value="'+esc(cells[a]??'')+'"></td>';}).join('')+'</tr>';
  }).join('');
  this.root.hidden=false;
  this.root.innerHTML='<div class="fusion-panel" role="dialog" aria-modal="true" aria-label="Edit embedded Sheets table"><div class="fusion-toolbar"><strong>Sheets · '+esc(obj.label)+'</strong><span>Embedded · formulas preserved</span><button id="fusion-prev">↑ Rows</button><button id="fusion-next">↓ Rows</button><button id="fusion-go">Go to cell</button><button id="fusion-close">Close</button></div><div class="fusion-grid-scroll"><table class="fusion-grid"><thead><tr><th></th>'+cols.map(c=>'<th>'+c+'</th>').join('')+'</tr></thead><tbody>'+grid+'</tbody></table></div><div class="fusion-help">Edit any cell or paste a TSV range. Use Go to cell to navigate A1–ZZ10000. The embedded workbook is stored inside Writer.</div></div>';
  this.root.querySelector<HTMLButtonElement>('#fusion-close')!.onclick=()=>this.close();
  this.root.querySelector<HTMLButtonElement>('#fusion-prev')!.onclick=()=>{this.startRow=Math.max(1,this.startRow-20);this.draw();};
  this.root.querySelector<HTMLButtonElement>('#fusion-next')!.onclick=()=>{this.startRow=Math.min(9981,this.startRow+20);this.draw();};
  this.root.querySelector<HTMLButtonElement>('#fusion-go')!.onclick=()=>{
   const raw=window.prompt('Go to cell (A1–ZZ10000)',address(this.startRow,this.startCol));
   if(!raw)return;
   const m=/^([A-Z]{1,2})([1-9]\d{0,4})$/.exec(raw.trim().toUpperCase());
   if(!m){this.notify('Invalid cell address');return;}
   const c=[...m[1]].reduce((n,x)=>n*26+x.charCodeAt(0)-64,0),row=Number(m[2]);
   if(c>702||row>10000){this.notify('Address exceeds ZZ10000');return;}
   this.startCol=Math.min(691,c);this.startRow=Math.min(9981,row);
   this.draw();
   this.root.querySelector<HTMLInputElement>('[data-fusion-cell="'+address(row,c)+'"]')?.focus();
  };
  this.root.querySelectorAll<HTMLInputElement>('[data-fusion-cell]').forEach(input=>{
   input.onchange=()=>{
    const key=input.dataset.fusionCell!;
    const current=this.current()?.workbook.sheets[0].cells[key]??'';
    if(input.value!==current)void this.commit({[key]:input.value});
   };
   input.onpaste=e=>{
    const raw=e.clipboardData?.getData('text/plain');
    if(!raw||!(/[\t\n\r]/.test(raw)))return;
    e.preventDefault();
    const match=/^([A-Z]+)(\d+)$/.exec(input.dataset.fusionCell!)!;
    const firstRow=Number(match[2]),firstCol=[...match[1]].reduce((n,c)=>n*26+c.charCodeAt(0)-64,0);
    const updates:Record<string,string>={};
    raw.replace(/\r\n?/g,'\n').replace(/\n$/,'').split('\n').forEach((line,r)=>line.split('\t').forEach((v,c)=>{
     if(firstRow+r<=10000&&firstCol+c<=702)updates[address(firstRow+r,firstCol+c)]=v;
    }));
    void this.commit(updates);
   };
  });
 }
}
