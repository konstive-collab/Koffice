import {type WriterPage,pageCss,DEFAULT_PAGE} from './writer-page';
import {sanitizeRichHtml,plaintextRichHtml} from './writer-rich';

/** Read-only pagination; the source editor and its selection are never mutated. */
export type WriterPagination={count:number;oversized:number;truncated:boolean};
const MAX_BLOCKS=1500;
const MAX_PAGES=250;
const MAX_SPLIT_TEXT=100_000;
const SPLITTABLE=new Set(['p','h1','h2','h3','blockquote','div']);
function create<K extends keyof HTMLElementTagNameMap>(name:K,className:string):HTMLElementTagNameMap[K]{
 const element=document.createElement(name);element.className=className;return element;
}
function setPageVars(element:HTMLElement,page:WriterPage){
 for(const [key,value] of Object.entries(pageCss(page)))element.style.setProperty(key,value);
}
function makePage(page:WriterPage,pageNumber:number,title:string){
 const paper=create('section','writer-preview-paper');
 paper.dataset.page=String(pageNumber);setPageVars(paper,page);
 paper.setAttribute('aria-label','Document page '+pageNumber);
 const header=create('header','writer-preview-page-header');header.textContent=page.header;
 const body=create('div','writer-preview-page-content writer-rich-editor');
 const footer=create('footer','writer-preview-page-footer');footer.textContent=page.footer;
 const number=create('div','writer-preview-page-number');number.textContent=title+' · '+pageNumber;
 paper.append(header,body,footer,number);
 return {paper,body};
}
function plainTextNodes(root:Element):Text[]{
 const result:Text[]=[];
 const walk=document.createTreeWalker(root,NodeFilter.SHOW_TEXT);
 let n:Node|null;
 while((n=walk.nextNode()))result.push(n as Text);
 return result;
}
/** Clone a text span with DOM Range, preserving inline tags inside paragraphs. */
export function copyFormattedRange(source:Element,start:number,end:number):Element{
 const nodes=plainTextNodes(source);
 const result=source.cloneNode(false) as Element;
 if(!nodes.length)return result;
 const total=nodes.reduce((n,node)=>n+node.length,0);
 if(start<0||end>total||start>end)throw Error('Invalid formatted text range');
 function point(offset:number):{node:Text;offset:number}{
  let remaining=offset;
  for(const node of nodes){
   if(remaining<=node.length)return {node,offset:remaining};
   remaining-=node.length;
  }
  return {node:nodes[nodes.length-1],offset:nodes[nodes.length-1].length};
 }
 const a=point(start),b=point(end);
 const range=document.createRange();
 range.setStart(a.node,a.offset);range.setEnd(b.node,b.offset);
 result.append(range.cloneContents());
 return result;
}
function safeSplitEnd(text:string,start:number,end:number):number{
 // Never divide UTF-16 surrogate pairs, which would corrupt emoji.
 if(end>start&&end<text.length){
  const a=text.charCodeAt(end-1),b=text.charCodeAt(end);
  if(a>=0xD800&&a<=0xDBFF&&b>=0xDC00&&b<=0xDFFF)end--;
 }
 return Math.max(start,end);
}
function splitPreferred(text:string,start:number,end:number):number{
 const safe=safeSplitEnd(text,start,end);
 if(safe<=start)return start;
 // Avoid excessive whitespace fragmentation when a word boundary is close.
 const last=text.lastIndexOf(' ',safe);
 return last>start+Math.max(5,Math.floor((safe-start)*.75))?last+1:safe;
}
/** Returns content-preserving snapshots, not edits to the source document. */
export function paginateWriterBlocks(
 source:HTMLElement,settings:WriterPage,title:string,
 onPage:(page:HTMLElement)=>void,measurement:HTMLElement
):WriterPagination{
 const measureBody=measurement.querySelector<HTMLElement>('.writer-preview-page-content');
 if(!measureBody)throw Error('Missing page measurement surface');
 let count=0,oversized=0,truncated=false;
 let current:ReturnType<typeof makePage>;
 let height=0;
 function beginPage():boolean{
  if(count>=MAX_PAGES){truncated=true;return false;}
  current=makePage(settings,count+1,title);
  onPage(current.paper);count++;
  measureBody.replaceChildren();
  // Computed height of the content slot, after margins/header/footer.
  height=measureBody.clientHeight;
  if(height<80)throw Error('Page is too small for Writer preview');
  return true;
 }
 function testFits(node:Node):boolean{
  const probe=node.cloneNode(true);
  measureBody.append(probe);
  const fits=measureBody.scrollHeight<=height+1;
  measureBody.removeChild(probe);
  return fits;
 }
 function append(node:Node){
  const forMeasure=node.cloneNode(true);
  measureBody.append(forMeasure);
  current.body.append(node.cloneNode(true));
 }
 function oversizedBlock(node:Node){
  if(current.body.childNodes.length && !beginPage())return false;
  append(node);oversized++;current.paper.classList.add('writer-page-overflow');
  return true;
 }
 function wholeBlock(node:Node):boolean{
  if(testFits(node)){append(node);return true;}
  if(current.body.childNodes.length){
   if(!beginPage())return false;
   if(testFits(node)){append(node);return true;}
  }
  return oversizedBlock(node);
 }
 // Measure from inside one off-screen, full-size physical page. The visible
 // preview can zoom independently without invalidating the line breaks.
 if(!beginPage())return {count,oversized,truncated};
 const blocks=Array.from(source.childNodes);
 for(let index=0;index<blocks.length;index++){
  if(index>=MAX_BLOCKS){truncated=true;break;}
  const block=blocks[index],el=block.nodeType===Node.ELEMENT_NODE?block as Element:null;
  const tag=el?.tagName.toLowerCase();
  if(tag==='section'){
   // Explicit page break. Empty paragraphs immediately after it stay intact.
   if(!beginPage())break;
   continue;
  }
  if(tag==='table'){
   if(!appendTable(el as HTMLTableElement))break;
   continue;
  }
  if(el&&tag&&SPLITTABLE.has(tag)&&el.textContent&&el.textContent.length<=MAX_SPLIT_TEXT&&!el.querySelector('table,ul,ol,section,hr,br')){
   if(!appendParagraph(el))break;
   continue;
  }
  if(!wholeBlock(block))break;
 }
 function appendParagraph(element:Element):boolean{
  const text=element.textContent??'';
  if(!text.length)return wholeBlock(element);
  let start=0;
  while(start<text.length){
   if(truncated)return false;
   const remainder=copyFormattedRange(element,start,text.length);
   if(testFits(remainder)){append(remainder);return true;}
   // Binary search the largest formatted text prefix that fits the page.
   let lo=start,hi=text.length;
   while(lo+1<hi){
    const mid=safeSplitEnd(text,start,Math.floor((lo+hi)/2));
    if(mid<=start){hi=Math.max(start+1,hi-1);continue;}
    if(testFits(copyFormattedRange(element,start,mid)))lo=mid;
    else hi=mid;
   }
   let end=splitPreferred(text,start,lo);
   if(end<=start){
    if(current.body.childNodes.length){if(!beginPage())return false;continue;}
    // A single oversize glyph/inline object must remain visible, not disappear.
    return oversizedBlock(copyFormattedRange(element,start,text.length));
   }
   const fragment=copyFormattedRange(element,start,end);
   append(fragment);start=end;
   if(start<text.length&&!beginPage())return false;
  }
  return true;
 }
 function appendTable(table:HTMLTableElement):boolean{
  const rows=Array.from(table.rows);
  if(!rows.length)return wholeBlock(table);
  const header=table.tHead?.cloneNode(true) as HTMLTableSectionElement|undefined;
  let first=true;
  let rowIndex=0;
  function tableChunk():HTMLTableElement{
   const chunk=table.cloneNode(false) as HTMLTableElement;
   if(header&&!first)chunk.append(header.cloneNode(true));
   const body=document.createElement('tbody');chunk.append(body);
   return chunk;
  }
  let chunk=tableChunk();
  while(rowIndex<rows.length){
   const row=rows[rowIndex];
   // Rows from the original thead are represented by the repeated header.
   if(table.tHead?.contains(row)){
    if(first){
     const head=table.tHead.cloneNode(true);
     chunk.insertBefore(head,chunk.firstChild);
    }
    rowIndex++;continue;
   }
   const body=chunk.tBodies[0]??chunk.createTBody();
   body.append(row.cloneNode(true));
   if(testFits(chunk)){
    rowIndex++;continue;
   }
   body.lastChild?.remove();
   if(body.rows.length || (first&&table.tHead)){
    append(chunk);
    if(!beginPage())return false;
    first=false;chunk=tableChunk();continue;
   }
   if(current.body.childNodes.length){if(!beginPage())return false;first=false;chunk=tableChunk();continue;}
   // An individual table row is indivisible in this stage.
   body.append(row.cloneNode(true));
   if(!oversizedBlock(chunk))return false;
   rowIndex++;
   if(rowIndex<rows.length){if(!beginPage())return false;first=false;chunk=tableChunk();}
  }
  if((chunk.tBodies[0]?.rows.length??0)>0 || (first&&table.tHead))return wholeBlock(chunk);
  return true;
 }
 return {count,oversized,truncated};
}
export function openWriterPages(page:WriterPage|undefined,richHtml:string|undefined|null,plain:string,title:string):void{
 const settings=page??DEFAULT_PAGE;
 const html=sanitizeRichHtml(richHtml??plaintextRichHtml(plain));
 const source=create('div','writer-page-source');source.innerHTML=html;
 const overlay=create('div','writer-pages-overlay');
 overlay.setAttribute('role','dialog');overlay.setAttribute('aria-modal','true');
 overlay.setAttribute('aria-label','Paginated Writer preview');
 const bar=create('div','writer-pages-toolbar');
 const heading=create('strong','writer-pages-heading');heading.textContent=title+' · Page preview';
 const previous=create('button','writer-pages-first');previous.textContent='Previous';previous.type='button';
 const next=create('button','writer-pages-next');next.textContent='Next';next.type='button';
 const counter=create('span','writer-pages-count');counter.setAttribute('aria-live','polite');
 const scale=create('select','writer-pages-zoom');scale.setAttribute('aria-label','Preview scale');
 for(const v of [50,65,80,100]){
  const option=document.createElement('option');option.value=String(v);option.textContent=v+'%';
  if(v===80)option.selected=true;scale.append(option);
 }
 const done=create('button','writer-pages-close');done.type='button';done.textContent='Edit document';
 const note=create('span','writer-pages-warning');
 bar.append(heading,previous,counter,next,scale,done);
 const viewport=create('div','writer-pages-scroll');
 const pages=create('div','writer-pages-collection');viewport.append(pages);
 overlay.append(bar,note,viewport);document.body.append(overlay);
 const measurement=create('section','writer-preview-paper writer-page-measure');
 setPageVars(measurement,settings);
 const measureHeader=create('header','writer-preview-page-header');measureHeader.textContent=settings.header;
 const measureBody=create('div','writer-preview-page-content writer-rich-editor');
 const measureFooter=create('footer','writer-preview-page-footer');measureFooter.textContent=settings.footer;
 measurement.append(measureHeader,measureBody,measureFooter);document.body.append(measurement);
 let selected=0;
 const built:HTMLElement[]=[];
 const close=()=>{overlay.remove();measurement.remove();};
 function show(index:number){
  selected=Math.max(0,Math.min(index,built.length-1));
  built.forEach((paper,i)=>paper.hidden=i!==selected);
  counter.textContent='Page '+(selected+1)+' of '+built.length;
  previous.disabled=selected===0;next.disabled=selected===built.length-1;
  viewport.scrollTop=0;
 }
 function zoom(){
  pages.style.setProperty('--writer-preview-zoom',String(Number(scale.value)/100));
 }
 previous.onclick=()=>show(selected-1);
 next.onclick=()=>show(selected+1);
 done.onclick=close;scale.onchange=zoom;
 overlay.addEventListener('keydown',e=>{
  if(e.key==='Escape'){e.stopPropagation();close();}
  if(e.key==='PageDown'&&!(e.target instanceof HTMLSelectElement)){e.preventDefault();show(selected+1);}
  if(e.key==='PageUp'&&!(e.target instanceof HTMLSelectElement)){e.preventDefault();show(selected-1);}
 });
 try{
  const result=paginateWriterBlocks(source,settings,title,paper=>{
   built.push(paper);pages.append(paper);
  },measurement);
  if(result.oversized||result.truncated)note.textContent=
   (result.oversized?result.oversized+' oversized blocks are scrollable. ':'')+
   (result.truncated?'Preview safety limit reached. ':'')+'Source document remains intact.';
  show(0);zoom();done.focus();
 }catch(error){close();throw error;}
}
