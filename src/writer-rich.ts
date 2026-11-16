/** Limited, reproducible HTML model for the Writer rich-text alpha.
 * Never render the raw rich_html from an opened .kdoc; strip all attributes,
 * event handlers, SVG, active content and unsupported tags first.
 */
const BLOCKS=new Set(['p','div','h1','h2','h3','blockquote','ul','ol','li','table','thead','tbody','tr','td','th']);
const INLINE=new Set(['strong','b','em','i','u','s','sub','sup','code','span']);
const VOID=new Set(['br','hr']);
const ALL=new Set([...BLOCKS,...INLINE,...VOID]);
export function escapeMarkup(value:string):string{
 return value.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
}
function safeNode(node:Node,doc:Document):Node{
 if(node.nodeType===Node.TEXT_NODE)return doc.createTextNode(node.textContent??'');
 if(node.nodeType!==Node.ELEMENT_NODE)return doc.createDocumentFragment();
 const el=node as Element;
 const tag=el.tagName.toLowerCase();
 // Drop the entire contents of dangerous element types.
 if(['script','style','iframe','svg','math','object','embed','form','input','video','audio','textarea','template'].includes(tag))return doc.createDocumentFragment();
 const wrapper=ALL.has(tag)?doc.createElement(tag):doc.createDocumentFragment();
 if(!VOID.has(tag)){
   for(const child of Array.from(el.childNodes))wrapper.append(safeNode(child,doc));
 }
 return wrapper;
}
export function sanitizeRichHtml(input:string):string{
 const parsed=new DOMParser().parseFromString(input,'text/html');
 const output=document.createElement('div');
 for(const child of Array.from(parsed.body.childNodes))output.append(safeNode(child,document));
 return output.innerHTML;
}
export function plaintextRichHtml(text:string):string{
 if(!text)return '<p><br></p>';
 return text.replace(/\r\n?/g,'\n').split('\n').map(line=>'<p>'+(line?escapeMarkup(line):'<br>')+'</p>').join('');
}
export function richTextContent(editor:HTMLElement):string{
 // innerText incorporates visual block boundaries; use plain fallback in headless tests.
 return (editor.innerText??editor.textContent??'').replace(/\r\n?/g,'\n');
}
export function readRichEditor(editor:HTMLElement):{text:string;html:string}{
 const html=sanitizeRichHtml(editor.innerHTML);
 const container=document.createElement('div');
 container.innerHTML=html;
 return {html,text:richTextContent(container)};
}
export type WriterFormat='bold'|'italic'|'underline'|'strike'|'h1'|'h2'|'paragraph'|'quote'|'bullet'|'number'|'date'|'line'|'table'|'undo'|'redo';
export function formatSelection(editor:HTMLElement,action:WriterFormat):void{
 editor.focus();
 const apply=(cmd:string,value?:string)=>document.execCommand(cmd,false,value);
 switch(action){
  case 'bold':apply('bold');break;
  case 'italic':apply('italic');break;
  case 'underline':apply('underline');break;
  case 'strike':apply('strikeThrough');break;
  case 'h1':apply('formatBlock','h1');break;
  case 'h2':apply('formatBlock','h2');break;
  case 'paragraph':apply('formatBlock','p');break;
  case 'quote':apply('formatBlock','blockquote');break;
  case 'bullet':apply('insertUnorderedList');break;
  case 'number':apply('insertOrderedList');break;
  case 'date':apply('insertText',new Date().toLocaleDateString());break;
  case 'line':apply('insertHorizontalRule');break;
  case 'table':{
   const cells='<tr><th>Heading 1</th><th>Heading 2</th></tr><tr><td>Cell 1</td><td>Cell 2</td></tr><tr><td>Cell 3</td><td>Cell 4</td></tr>';
   apply('insertHTML','<table><tbody>'+cells+'</tbody></table><p><br></p>');
   break;
  }
  case 'undo':apply('undo');break;
  case 'redo':apply('redo');break;
 }
 editor.dispatchEvent(new Event('input',{bubbles:true}));
}
export function replaceEditorMatch(editor:HTMLElement,needle:string,replacement:string):boolean{
 const walker=document.createTreeWalker(editor,NodeFilter.SHOW_TEXT);
 let node:Node|null;
 while((node=walker.nextNode())){
  const text=node.textContent??'';
  const index=text.indexOf(needle);
  if(index<0)continue;
  const range=document.createRange();
  range.setStart(node,index);range.setEnd(node,index+needle.length);
  const selection=window.getSelection();
  selection?.removeAllRanges();selection?.addRange(range);
  editor.focus();
  if(replacement!==needle){range.deleteContents();range.insertNode(document.createTextNode(replacement));editor.dispatchEvent(new Event('input',{bubbles:true}));}
  return true;
 }
 return false;
}
