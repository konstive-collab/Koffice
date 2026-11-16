import {type WriterPage,paperDimensions,pageCss,escapePageText,DEFAULT_PAGE} from './writer-page';
import {sanitizeRichHtml,plaintextRichHtml} from './writer-rich';

/**
 * Read-only paginated Writer preview.
 * Keeps the contenteditable DOM and selection completely untouched.
 * Measures actual rendered block heights in a hidden, style-identical page.
 * Oversize indivisible blocks remain scrollable and are reported, never lost.
 */
export type WriterPagination={count:number;oversized:number;truncated:boolean};
const MAX_BLOCKS=1500;
const MAX_PAGES=250;
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
 const header=create('header','writer-preview-page-header');
 header.textContent=page.header;
 const body=create('div','writer-preview-page-content writer-rich-editor');
 const footer=create('footer','writer-preview-page-footer');
 footer.textContent=page.footer;
 const number=create('div','writer-preview-page-number');
 number.textContent=title+' · '+pageNumber;
 paper.append(header,body,footer,number);
 return {paper,body};
}
export function openWriterPages(
 page:WriterPage|undefined,richHtml:string|undefined|null,plain:string,title:string
):void{
 const settings=page??DEFAULT_PAGE;
 const html=sanitizeRichHtml(richHtml??plaintextRichHtml(plain));
 const raw=document.createElement('div');
 raw.innerHTML=html;
 const overlay=create('div','writer-pages-overlay');
 overlay.setAttribute('role','dialog');overlay.setAttribute('aria-modal','true');
 overlay.setAttribute('aria-label','Paginated Writer preview');
 const bar=create('div','writer-pages-toolbar');
 const heading=create('strong','writer-pages-heading');heading.textContent=title+' · Page preview';
 const first=create('button','writer-pages-first');first.textContent='Previous';first.type='button';
 const next=create('button','writer-pages-next');next.textContent='Next';next.type='button';
 const counter=create('span','writer-pages-count');counter.setAttribute('aria-live','polite');
 const scale=create('select','writer-pages-zoom');scale.setAttribute('aria-label','Preview scale');
 for(const v of [50,65,80,100]){
  const option=document.createElement('option');option.value=String(v);option.textContent=v+'%';
  if(v===80)option.selected=true;scale.append(option);
 }
 const done=create('button','writer-pages-close');done.type='button';done.textContent='Edit document';
 const note=create('span','writer-pages-warning');
 bar.append(heading,first,counter,next,scale,done);
 const viewport=create('div','writer-pages-scroll');
 const pages=create('div','writer-pages-collection');viewport.append(pages);
 overlay.append(bar,note,viewport);
 document.body.append(overlay);
 const measurement=create('section','writer-preview-paper writer-page-measure');
 setPageVars(measurement,settings);
 const measureBody=create('div','writer-preview-page-content writer-rich-editor');
 const measureHeader=create('header','writer-preview-page-header');measureHeader.textContent=settings.header;
 const measureFooter=create('footer','writer-preview-page-footer');measureFooter.textContent=settings.footer;
 measurement.append(measureHeader,measureBody,measureFooter);document.body.append(measurement);

 let selected=0;
 const built:HTMLElement[]=[];
 let oversized=0,truncated=false;
 const {height}=paperDimensions(settings);
 // Header/footer geometry and vertical printable space are measured using
 // the actual page body. This avoids hard-coding pixel values or viewport DPI.
 const measureHeight=()=>{
  const width=measurement.getBoundingClientRect().width;
  if(width<10)return 0;
  return Math.max(80,measureBody.clientHeight);
 };
 // Deliberately preserve large blocks as scrollable; never silently truncate
 // a table or rich paragraph, which would lose data in print preview.
 function build(){
  let current=makePage(settings,1,title);
  built.push(current.paper);pages.append(current.paper);
  measureBody.replaceChildren();
  const contentHeight=Math.max(80,measureHeight());
  const blocks=Array.from(raw.childNodes);
  if(!blocks.length){current.body.append(document.createElement('p'));}
  for(let index=0;index<blocks.length;index++){
   if(index>=MAX_BLOCKS || built.length>=MAX_PAGES){truncated=true;break;}
   const block=blocks[index];
   const manual=block.nodeType===Node.ELEMENT_NODE&&(block as Element).tagName.toLowerCase()==='section';
   if(manual){
     if(current.body.childNodes.length>0){
       current=makePage(settings,built.length+1,title);
       built.push(current.paper);pages.append(current.paper);
       measureBody.replaceChildren();
     }
     continue;
   }
   const clone=block.cloneNode(true);
   measureBody.append(clone);
   const rect=measureBody.getBoundingClientRect();
   const measured=Math.max(0,rect.height);
   // The measurement container is fixed-height: scrollHeight is the real
   // cumulative content height, including margin collapse and tables.
   const required=Math.max(measured,measureBody.scrollHeight);
   if(required>contentHeight && current.body.childNodes.length>0){
    current=makePage(settings,built.length+1,title);
    built.push(current.paper);pages.append(current.paper);
    measureBody.replaceChildren();
    measureBody.append(clone);
   }
   const blockHeight=Math.max(measureBody.scrollHeight,clone.nodeType===Node.ELEMENT_NODE?(clone as Element).getBoundingClientRect().height:0);
   if(blockHeight>contentHeight && current.body.childNodes.length===0){
     oversized++;
     current.paper.classList.add('writer-page-overflow');
   }
   current.body.append(block.cloneNode(true));
  }
  if(truncated){
   const warning=create('p','writer-preview-cutoff');
   warning.textContent='Preview limit reached. The source document was not changed; use the editor for remaining content.';
   current.body.append(warning);
  }
 }
 const close=()=>{overlay.remove();measurement.remove();};
 function show(index:number){
  selected=Math.max(0,Math.min(index,built.length-1));
  built.forEach((paper,i)=>{paper.hidden=i!==selected;});
  counter.textContent='Page '+(selected+1)+' of '+built.length;
  first.disabled=selected===0;next.disabled=selected===built.length-1;
  viewport.scrollTop=0;
 }
 function zoom(){
  const factor=Number(scale.value)/100;
  pages.style.setProperty('--writer-preview-zoom',String(factor));
 }
 first.onclick=()=>show(selected-1);
 next.onclick=()=>show(selected+1);
 done.onclick=close;
 scale.onchange=zoom;
 overlay.addEventListener('keydown',e=>{
  if(e.key==='Escape'){e.stopPropagation();close();}
  if(e.key==='PageDown' && !(e.target instanceof HTMLSelectElement)){e.preventDefault();show(selected+1);}
  if(e.key==='PageUp' && !(e.target instanceof HTMLSelectElement)){e.preventDefault();show(selected-1);}
 });
 try {
   build();
   if(oversized || truncated)note.textContent=(oversized?oversized+' oversized blocks are scrollable. ':'')+
    (truncated?'Preview safety limit reached. ':'')+'Source document remains intact.';
   const count:WriterPagination={count:built.length,oversized,truncated};
   void count;
   show(0);zoom();done.focus();
 }catch(error){close();throw error;}
}
