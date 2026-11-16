import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import './style.css';
import {sanitizeRichHtml,plaintextRichHtml,readRichEditor,formatSelection,replaceEditorMatch,type WriterFormat} from './writer-rich';
import {DEFAULT_PAGE,pageCss,cssPageStyle,escapePageText,pagePrintableHtml,openPageSetup,type WriterPage} from './writer-page';
import {openWriterPages} from './writer-pagination';
import {colName,parseCellAddress,parseRange,viewportAround,nextViewport,type GridViewport} from './sheets-workbench';
import { BaseUI } from './base-ui';
import { FusionUI, type EmbeddedSheet } from './fusion-ui';
import { FusionManagerUI, type LinkNode, type PreviewReport } from './fusion-manager-ui';
import {renderSlideFrames,renderStaticSlideFrames,wireSlideFrames,type SlideObjectFrame} from './slide-layout';

type Kind='writer'|'sheets'|'slides'|'base';
type DesktopMode=Kind|'suite';
let desktopMode:DesktopMode='suite';
type Doc={format:string;title:string;text:string;rich_html?:string|null;page?:WriterPage;revision:number;embedded_objects?:EmbeddedSheet[]};
type Book={format:string;title:string;revision:number;sheets:{name:string;cells:Record<string,string>}[]};
type BaseSlideSource={database_path:string;table:string;clean:{title:string;body:string;notes:string}};
type Slide={title:string;body:string;notes:string;base_source?:BaseSlideSource;embedded_objects?:EmbeddedSheet[];object_frames?:Record<string,SlideObjectFrame>};
type Deck={format:string;title:string;revision:number;slides:Slide[]};
let active:Kind='writer',doc:Doc,book:Book,deck:Deck,slideIndex=0,sheetIndex=0;
let gridViewport:GridViewport={row:1,col:1,rows:40,cols:16};
const paths:Record<Kind,string|null>={writer:null,sheets:null,slides:null,base:null};
const saved:Record<Kind,number>={writer:0,sheets:0,slides:0,base:0};
let dirty=false;
let queue=Promise.resolve();
const run=(fn:()=>Promise<void>)=>{queue=queue.then(fn).catch(e=>message(String(e)));return queue;};
const $=(id:string)=>document.getElementById(id)!;
const escapeHtml=(s:string)=>s.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const revision=()=>active==='writer'?doc.revision:active==='sheets'?book.revision:active==='slides'?deck.revision:0;
const title=()=>active==='writer'?doc.title:active==='sheets'?book.title:active==='slides'?deck.title:'Database';
function message(t:string){$('notice').textContent=t;$('notice').classList.add('show');setTimeout(()=>$('notice').classList.remove('show'),5000);}
function confirmDiscard(){return !dirty&&saved[active]===revision()||window.confirm('Discard unsaved changes in this workspace?');}
const extensions:Record<Kind,string>={writer:'kdoc',sheets:'ksheet',slides:'kslide',base:'kdb'};
const labels:Record<Kind,string>={writer:'Writer',sheets:'Sheets',slides:'Slides',base:'Base'};
const source='Koffice · Fast · Local · Cross-platform';
const PREF_KEY='koffice.desktop.preferences.v1';
type Preferences={compact:boolean;scale:number};
function loadPreferences():Preferences{
  try {
    const raw=JSON.parse(localStorage.getItem(PREF_KEY)||'{}') as Partial<Preferences>;
    return {compact:raw.compact===true,scale:typeof raw.scale==='number'&&Number.isFinite(raw.scale)?Math.max(.8,Math.min(1.35,raw.scale)):1};
  }catch{return {compact:false,scale:1};}
}
const preferences=loadPreferences();
function applyPreferences(){
  document.documentElement.style.setProperty('--content-scale',String(preferences.scale));
  document.body.classList.toggle('compact-layout',preferences.compact);
  const compact=$('toggle-compact');if(compact)compact.setAttribute('aria-pressed',String(preferences.compact));
  const zoom=$('zoom-value');if(zoom)zoom.textContent=Math.round(preferences.scale*100)+'%';
  try{localStorage.setItem(PREF_KEY,JSON.stringify(preferences));}catch{ /* Preferences remain usable without storage. */ }
}
let searchNeedle='';
function toAddress(row:number,col:number){let letters='';while(col>0){col--;letters=String.fromCharCode(65+col%26)+letters;col=Math.floor(col/26);}return letters+row;}
function parseCsv(input:string):string[][]{const rows:string[][]=[];let row:string[]=[],value='',quoted=false;for(let i=0;i<input.length;i++){const ch=input[i];if(quoted){if(ch==='\"'&&input[i+1]==='\"'){value+='\"';i++;}else if(ch==='\"'){quoted=false;}else value+=ch;}else if(ch==='\"'&&value===''){quoted=true;}else if(ch===','){row.push(value);value='';}else if(ch==='\n'||ch==='\r'){if(ch==='\r'&&input[i+1]==='\n')i++;row.push(value);value='';rows.push(row);row=[];}else value+=ch;}if(quoted)throw Error('Unclosed CSV quote');if(row.length||value){row.push(value);rows.push(row);}return rows;}

const app=document.querySelector<HTMLDivElement>('#app')!;
app.innerHTML=`<aside class="sidebar"><div class="logo"><div class="logo-mark">K</div><div><b>Koffice</b><small>COMMUNITY ALPHA</small></div></div><span class="heading">WORKSPACE</span><button class="nav" id="nav-writer">▤ &nbsp; Writer <span>Text</span></button><button class="nav" id="nav-sheets">▦ &nbsp; Sheets <span>Data</span></button><button class="nav" id="nav-slides">▣ &nbsp; Slides <span>Present</span></button><button class="nav" id="nav-base">▥ &nbsp; Base <span>Database</span></button><div class="side-foot"><div class="dot"></div><span>Running locally<br><small>${source}</small></span></div></aside><main class="main"><header class="header"><div><div class="breadcrumb">Your workspace <span> / </span> <span id="kind">Writer</span></div><div class="document-name" id="docname">Untitled</div></div><div class="version">v0.21 · KODI OFFICE</div></header><div class="toolbar"><button id="open-apps" title="Launch a Kodi Office application">◫ Applications</button><div class="sep"></div><button id="new">＋ New</button><button id="open">↥ Open</button><button id="save">↧ Save</button><button id="saveas">Save as</button><div class="sep"></div><button id="find">Find / Replace</button><button id="fusion-center">Fusion links</button><button id="automation">Automation API</button><button id="export">Export</button><button id="print">Print / PDF</button><div class="spacer"></div><button id="toggle-compact" title="Show or hide sidebar" aria-label="Toggle sidebar">☰</button><button id="zoom-out" aria-label="Zoom out">−</button><span id="zoom-value" aria-live="polite">100%</span><button id="zoom-in" aria-label="Zoom in">＋</button><span class="status" id="status">Ready</span></div><section class="canvas" id="canvas"></section><footer><span id="details">Koffice</span><span id="location">Unsaved workspace</span><span id="rev">Revision 0</span></footer><div id="notice" role="status"></div></main>`;
/** Standalone editors run as different native processes, not merely tabs. */
const appLauncher=document.createElement('div');
appLauncher.className='desktop-launcher';
appLauncher.id='desktop-launcher';
appLauncher.hidden=true;
appLauncher.innerHTML='<section class="desktop-launcher-panel" role="dialog" aria-modal="true" aria-label="Kodi Office applications">'+
 '<header class="desktop-launcher-header"><div><div class="desktop-launcher-kicker">KODI OFFICE</div>'+
 '<h2>Choose an application</h2><p>Each editor opens as its own desktop process and independent workspace.</p></div>'+
 '<button id="app-launcher-close" aria-label="Close launcher">✕</button></header>'+
 '<div class="desktop-app-grid">'+
 '<button class="desktop-app-tile" data-launch-kind="writer"><span class="desktop-app-icon">▤</span><strong>Kodi Writer</strong><small>Documents and page editing</small></button>'+
 '<button class="desktop-app-tile" data-launch-kind="sheets"><span class="desktop-app-icon">▦</span><strong>Kodi Sheets</strong><small>Spreadsheets and formulas</small></button>'+
 '<button class="desktop-app-tile" data-launch-kind="slides"><span class="desktop-app-icon">▣</span><strong>Kodi Slides</strong><small>Presentations and layout</small></button>'+
 '<button class="desktop-app-tile" data-launch-kind="base"><span class="desktop-app-icon">▥</span><strong>Kodi Base</strong><small>Relational databases</small></button></div>'+
 '<footer class="desktop-launcher-foot"><p>Integrated Suite retains the existing Fusion workflow. Independent apps exchange saved files, not live in-memory state.</p>'+
 '<button id="app-launcher-continue">Continue in Suite</button></footer></section>';
document.body.append(appLauncher);
function showAppLauncher(){appLauncher.hidden=false;appLauncher.querySelector<HTMLButtonElement>('[data-launch-kind]')?.focus();}
function hideAppLauncher(){appLauncher.hidden=true;}
for(const button of Array.from(appLauncher.querySelectorAll<HTMLButtonElement>('[data-launch-kind]'))){
  button.onclick=()=>void run(async()=>{
    const kind=button.dataset.launchKind as Kind;
    const name=await invoke<string>('desktop_launch',{kind});
    message('Launched '+name+' as a separate application');
    hideAppLauncher();
  });
}
$('app-launcher-close').onclick=hideAppLauncher;
$('app-launcher-continue').onclick=hideAppLauncher;
$('open-apps').onclick=showAppLauncher;
appLauncher.addEventListener('keydown',event=>{if(event.key==='Escape'){event.stopPropagation();hideAppLauncher();}});
function applyDesktopMode(mode:DesktopMode){
  desktopMode=mode;
  document.body.dataset.appMode=mode;
  document.title=mode==='suite'?'Koffice Suite':'Kodi '+labels[mode];
  if(mode!=='suite'){
    active=mode;
    $('open-apps').setAttribute('aria-label','Launch another Kodi Office application');
    $('fusion-center').hidden=mode!=='writer';
    $('automation').hidden=true;
    $('find').hidden=mode!=='writer';
  }
}

const fusion=new FusionUI(
  ()=>doc?.embedded_objects??[],
  updatesCallback,
  async id=>{await run(async()=>{await flush();doc=await invoke<Doc>('writer_remove_embedded',{expectedRevision:doc.revision,objectId:id});syncDirty();refresh();});},
  async id=>{await run(async()=>{
    await flush();
    book=await invoke<Book>('writer_embedded_to_sheets',{objectId:id,expectedRevision:book.revision});
    sheetIndex=book.sheets.length-1;active='sheets';dirty=true;refresh();
    message('Embedded sheet appended to Sheets with all formulas. Save the workbook to retain it.');
  });},
  async id=>{await run(async()=>{
    await flush();
    const state=await invoke<{state:string;detail:string;source_title:string;source_sheet:string}>('writer_link_status',{objectId:id});
    message('Link '+state.state+': '+state.detail);
  });},
  async id=>{await run(async()=>{
    await flush();
    const state=await invoke<{state:string;detail:string}>('writer_link_status',{objectId:id});
    if(state.state!=='changed'){message('Link '+state.state+': '+state.detail);return;}
    doc=await invoke<Doc>('writer_refresh_link',{expectedRevision:doc.revision,objectId:id});
    syncDirty();refresh();message('Linked table updated from current Sheets session. Save Writer to persist.');
  });},
  async id=>{await run(async()=>{
    await flush();
    const sheet=book.sheets[sheetIndex];
    if(!sheet)throw Error('Open a source worksheet in Sheets first');
    if(!window.confirm('Reconnect this embedded table to the currently selected sheet: '+book.title+' / '+sheet.name+'? This will replace the last synced copy, but never local edits.'))return;
    doc=await invoke<Doc>('writer_reconnect_link',{expectedRevision:doc.revision,objectId:id,sourceSheet:sheetIndex});
    syncDirty();refresh();message('Explicitly reconnected to selected Sheets worksheet.');
  });},
  async id=>{await run(async()=>{
    await flush();
    if(!window.confirm('Detach this table? The current cells and formulas will remain in Writer, but updates from Sheets will stop.'))return;
    doc=await invoke<Doc>('writer_detach_link',{expectedRevision:doc.revision,objectId:id});
    syncDirty();refresh();message('Link detached; embedded data retained.');
  });},
  async id=>{await run(async()=>{
    await flush();
    const link=doc.embedded_objects?.find(o=>o.id===id)?.file_link;
    if(!link)throw Error('Not linked to a file');
    if(!window.confirm('Check source file?\n'+link.source_path+'\nKoffice will read this file only for this requested check.'))return;
    const result=await invoke<{state:string;detail:string}>('writer_file_link_status',{objectId:id});
    message('File link '+result.state+': '+result.detail);
  });},
  async id=>{await run(async()=>{
    await flush();
    const link=doc.embedded_objects?.find(o=>o.id===id)?.file_link;
    if(!link)throw Error('Not linked to a file');
    if(!window.confirm('Check and refresh from this file?\n'+link.source_path+'\nLocal edits will block replacement.'))return;
    const status=await invoke<{state:string;detail:string}>('writer_file_link_status',{objectId:id});
    if(status.state!=='changed'){message('File link '+status.state+': '+status.detail);return;}
    doc=await invoke<Doc>('writer_refresh_file_link',{expectedRevision:doc.revision,objectId:id});
    syncDirty();refresh();message('Updated from disk. Save Writer to retain the new snapshot.');
  });},
  async id=>{await run(async()=>{
    await flush();
    const selection=await chooseFusionFile();
    if(!selection)return;
    if(!window.confirm('Replace the source of this file-linked table with:\n'+selection.path+'\nOnly a clean embedded copy can be reconnected.'))return;
    doc=await invoke<Doc>('writer_reconnect_file_link',{expectedRevision:doc.revision,objectId:id,path:selection.path,sourceSheet:selection.sourceSheet});
    syncDirty();refresh();message('File link reconnected. Save Writer to persist.');
  });},
  async id=>{await run(async()=>{
    await flush();
    if(!window.confirm('Detach this file link and retain all embedded cells and formulas?'))return;
    doc=await invoke<Doc>('writer_detach_file_link',{expectedRevision:doc.revision,objectId:id});
    syncDirty();refresh();message('File link detached; embedded sheet remains editable.');
  });},
  message
);
async function chooseFusionFile():Promise<{path:string;sourceSheet:number}|null>{
  const path=await open({multiple:false,filters:[{name:'Koffice Sheets workbook',extensions:['ksheet']}]});
  if(typeof path!=='string')return null;
  const sheets=await invoke<string[]>('fusion_file_sheets',{path});
  if(!sheets.length)throw Error('Source workbook has no worksheets');
  const selected=window.prompt('Source worksheet number:\n'+sheets.map((name,i)=>String(i+1)+'. '+name).join('\n'),'1');
  if(selected===null)return null;
  if(!/^[1-9]\d*$/.test(selected.trim()))throw Error('Enter a valid worksheet number');
  const index=Number(selected.trim())-1;
  if(index<0||index>=sheets.length)throw Error('Worksheet number out of range');
  return {path,sourceSheet:index};
}
async function updatesCallback(id:string,updates:Record<string,string>){
  await run(async()=>{
    await flush();
    doc=await invoke<Doc>('writer_embedded_put',{expectedRevision:doc.revision,objectId:id,updates});
    syncDirty();refresh();
    const first=Object.keys(updates)[0];
    if(first&&updates[first]?.startsWith('=')){
      try{message(first+' = '+await invoke<string>('writer_embedded_value',{objectId:id,address:first}));}catch(e){message(String(e));}
    }
  });
}
// One shared cross-app link center. Inventory never accesses file paths.
// The very same Sheets grid component hosts tables in Writer and Slides.
const unavailableSlideLink=async (_id:string)=>{throw new Error('Slides supports independent embedded tables in this release');};
const slideFusion=new FusionUI(
  ()=>deck?.slides[slideIndex]?.embedded_objects??[],
  async(id,updates)=>{await run(async()=>{
    await flush();
    deck=await invoke<Deck>('slides_embedded_put',{expectedRevision:deck.revision,index:slideIndex,objectId:id,updates});
    syncDirty();refresh();
    const first=Object.keys(updates)[0];
    if(first&&updates[first].startsWith('=')){
      try{message(first+' = '+await invoke<string>('slides_embedded_value',{index:slideIndex,objectId:id,address:first}));}
      catch(e){message(String(e));}
    }
  });},
  async id=>{await run(async()=>{
    await flush();
    if(!window.confirm('Remove this embedded table from the slide?'))return;
    deck=await invoke<Deck>('slides_embedded_remove',{expectedRevision:deck.revision,index:slideIndex,objectId:id});
    syncDirty();refresh();
  });},
  async id=>{await run(async()=>{
    await flush();
    book=await invoke<Book>('slides_embedded_to_sheets',{index:slideIndex,objectId:id,expectedRevision:book.revision});
    sheetIndex=book.sheets.length-1;active='sheets';syncDirty();refresh();
    message('Slides worksheet appended to Sheets. Save the workbook to retain it.');
  });},
  unavailableSlideLink,unavailableSlideLink,unavailableSlideLink,unavailableSlideLink,
  unavailableSlideLink,unavailableSlideLink,unavailableSlideLink,unavailableSlideLink,
  message
);
const fusionManager=new FusionManagerUI(
  ()=>invoke<LinkNode[]>('fusion_graph'),
  ids=>invoke<PreviewReport>('fusion_preview_refresh',{objectIds:ids}),
  async report=>{
    await flush();
    doc=await invoke<Doc>('fusion_apply_refresh',{token:report.token,expectedRevision:report.expected_revision});
    active='writer';syncDirty();refresh();
  },
  message
);
const base = new BaseUI(message,()=>refresh(),async(table)=>{
  if (saved.sheets!==book.revision && !window.confirm('Replace the unsaved Sheets workbook with the database snapshot?')) return;
  const next=await invoke<Book>('base_to_workbook',{table});
  book=next;paths.sheets=null;saved.sheets=0;sheetIndex=0;active='sheets';dirty=true;refresh();
  message('Database snapshot copied to Sheets; save the new .ksheet workbook.');
},async(table)=>{
  deck=await invoke<Deck>('base_snapshot_to_slides',{table,expectedRevision:deck.revision});
  slideIndex=deck.slides.length-1;active='slides';dirty=true;refresh();
  message('Static Base table snapshot appended to Slides. Save the presentation to retain it.');
},async(table)=>{
  if(!base.path)throw Error('Open a database first');
  if(!window.confirm('Create a linked slide using this source database?\n'+base.path+'\nThe slide will store a complete snapshot. Future reads require confirmation.'))return;
  deck=await invoke<Deck>('base_link_to_slides',{table,expectedRevision:deck.revision});
  slideIndex=deck.slides.length-1;active='slides';syncDirty();refresh();
  message('Base-linked slide inserted. Save Slides to persist the link and snapshot.');
});
function refresh(){document.querySelectorAll('.nav').forEach(x=>x.classList.remove('selected'));$(`nav-${active}`).classList.add('selected');$('kind').textContent=labels[active];$('docname').textContent=active==='base'?(base.path?.split(/[\\/]/).pop()??'Database'):title();$('rev').textContent=`Revision ${revision()}`;$('location').textContent=active==='base'?(base.path??'No database selected'):(paths[active]??'No file selected');$('status').textContent=active==='base'?'✓ SQLite · committed automatically':((dirty||saved[active]!==revision())?'● Unsaved changes':'✓ Saved locally');renderCanvas();}
function syncDirty(){dirty=true;$('status').textContent='● Unsaved changes';}
function renderCanvas(){
 if(active==='base'){base.render();$('details').textContent=`${base.tables.length} tables · SQLite`;return;}
 if(active==='writer') { $('canvas').innerHTML=`<div class="writer-area"><div class="formatbar writer-ribbon"><div class="writer-ribbon-group"><button data-format="undo" title="Undo">↶</button><button data-format="redo" title="Redo">↷</button></div><div class="writer-ribbon-group"><button data-format="bold" title="Bold (Ctrl+B)"><b>B</b></button><button data-format="italic" title="Italic (Ctrl+I)"><i>I</i></button><button data-format="underline" title="Underline (Ctrl+U)"><u>U</u></button><button data-format="strike" title="Strikethrough"><s>S</s></button></div><div class="writer-ribbon-group"><button data-format="paragraph">Body</button><button data-format="h1">Heading 1</button><button data-format="h2">Heading 2</button><button data-format="quote">Quote</button></div><div class="writer-ribbon-group"><button data-format="bullet">• List</button><button data-format="number">1. List</button><button data-format="table">Table</button><button data-format="date">Date</button><button data-format="line">Divider</button><button data-format="pagebreak">Page break</button></div><button id="writer-page-setup">Page setup</button><button id="writer-pages" title="Measured multi-page preview">Pages</button><button id="writer-find">Find / Replace</button><button id="writer-embed">Insert Sheets table</button><button id="writer-link">Link active Sheets</button><button id="writer-link-file">Link Sheets file</button><span class="hint">Rich text · page view · formatting saved in .kdoc</span></div><div class="paper" id="writer-paper"><div class="writer-page-header">${escapePageText((doc.page??DEFAULT_PAGE).header)}</div><input class="paper-title" id="writer-title" maxlength="200" aria-label="Document title" value="${escapeHtml(doc.title)}"><div class="paper-meta">KOFFICE WRITER · YOUR DOCUMENT</div><div id="text" class="writer-rich-editor" role="textbox" aria-multiline="true" aria-label="Document content" contenteditable="true" spellcheck="true" data-placeholder="Every great idea starts with a blank page..."></div><div class="fusion-document-objects">${fusion.preview()}</div><div class="writer-page-footer">${escapePageText((doc.page??DEFAULT_PAGE).footer)}</div></div></div>`; const el=$('text') as HTMLDivElement; el.innerHTML=sanitizeRichHtml(doc.rich_html??plaintextRichHtml(doc.text)); for(const [key,value] of Object.entries(pageCss(doc.page??DEFAULT_PAGE)))$('writer-paper').style.setProperty(key,value); $('details').textContent=`${doc.text.trim()?doc.text.trim().split(/\s+/u).length:0} words · ${[...doc.text].length} characters`;
  let timer:ReturnType<typeof setTimeout>|undefined;
  el.oninput=()=>{syncDirty();clearTimeout(timer);timer=setTimeout(()=>void commitWriter(),360);};
  el.onblur=()=>{clearTimeout(timer);void commitWriter();};
  ($('writer-title') as HTMLInputElement).onchange=()=>void run(async()=>{await commitWriter();let v=($('writer-title') as HTMLInputElement).value; if(v===doc.title)return;doc=await invoke('rename_document',{expectedRevision:doc.revision,title:v});syncDirty();refresh();});
  $('writer-find').onclick=()=>findReplace();
  $('writer-link-file').onclick=()=>void run(async()=>{
    await flush();
    const selection=await chooseFusionFile();
    if(!selection)return;
    const label=window.prompt('Label for file-linked table','Linked worksheet');
    if(!label)return;
    doc=await invoke<Doc>('writer_attach_file_sheet',{expectedRevision:doc.revision,label,path:selection.path,sourceSheet:selection.sourceSheet});
    syncDirty();refresh();message('File-linked table embedded. Save Writer to persist the link.');
  });
  $('writer-link').onclick=()=>void run(async()=>{
    await flush();
    const sheet=book.sheets[sheetIndex];
    if(!sheet)throw Error('Open a worksheet in Sheets first');
    const label=window.prompt('Name of linked table',sheet.name);
    if(!label)return;
    doc=await invoke<Doc>('writer_embed_linked_sheet',{expectedRevision:doc.revision,label,sourceSheet:sheetIndex});
    syncDirty();refresh();
    message('Session link created. Updates require manual Refresh; restart needs Reconnect.');
  });
  $('writer-embed').onclick=()=>void run(async()=>{
    await flush();
    const useCurrent=window.confirm('Copy the currently active Sheets worksheet? Cancel creates a new blank embedded sheet.');
    const label=window.prompt('Embedded table name',useCurrent?book.sheets[sheetIndex].name:'Embedded table');
    if(!label)return;
    doc=await invoke<Doc>('writer_embed_sheet',{expectedRevision:doc.revision,label,sourceSheet:useCurrent?sheetIndex:null});
    syncDirty();refresh();message('Editable Sheets table inserted; remember to save Writer.');
  });
  $('writer-pages').onclick=()=>{
    // Inspect the live unsaved editor content without moving its DOM nodes,
    // changing its selection or triggering an intermediate Save.
    const live=$('text') as HTMLDivElement;
    openWriterPages(doc.page,live.innerHTML,doc.text,doc.title);
  };
  $('writer-page-setup').onclick=()=>openPageSetup(doc.page??DEFAULT_PAGE,async page=>{
    await flush();
    doc=await invoke<Doc>('writer_set_page',{expectedRevision:doc.revision,page});
    syncDirty();refresh();message('Page setup changed. Save the document to retain it.');
  });
  fusion.wirePreview();
  document.querySelectorAll<HTMLButtonElement>('[data-format]').forEach(btn=>{
    btn.onmousedown=event=>event.preventDefault(); // preserve selected text on toolbar click
    btn.onclick=()=>formatSelection(el,btn.dataset.format as WriterFormat);
  });
 } else if(active==='sheets'){
   const b=book.sheets[sheetIndex];const cols=Array.from({length:gridViewport.cols},(_,i)=>colName(gridViewport.col+i));
   $('canvas').innerHTML=`<div class="sheet-pane"><div class="sheet-head"><b>Spreadsheet</b><span>Grid: 10,000 rows × 702 columns</span><button id="sheet-go-to">Go to cell</button><button id="sheet-sort">Sort range</button><button id="sheet-fill">Fill series</button><button id="sheet-clear">Clear range</button><button id="import-csv">Import CSV</button><button id="find-cells">Find cells</button><button id="sheet-stats">Analyze</button><button id="rename-sheet">Rename</button><button id="delete-sheet">Delete</button><button id="add-sheet">＋ Add sheet</button><input id="csv-picker" type="file" accept=".csv,text/csv" hidden></div><div class="grid-scroll"><table class="grid"><thead><tr><th class="rowhead"></th>${cols.map(c=>`<th>${c}</th>`).join('')}</tr></thead><tbody>${Array.from({length:gridViewport.rows},(_,r)=>`<tr><th class="rowhead">${gridViewport.row+r}</th>${cols.map(c=>`<td><input spellcheck="false" data-address="${c}${gridViewport.row+r}" value="${escapeHtml(b.cells[`${c}${gridViewport.row+r}`]??'')}" aria-label="${c}${gridViewport.row+r}"></td>`).join('')}</tr>`).join('')}</tbody></table></div><div class="sheet-navigation"><button id="sheet-prev-cols">◀ Columns</button><button id="sheet-next-cols">Columns ▶</button><button id="sheet-prev-rows">▲ Rows</button><button id="sheet-next-rows">Rows ▼</button><span>${cols[0]}${gridViewport.row}–${cols[cols.length-1]}${Math.min(10000,gridViewport.row+gridViewport.rows-1)}</span></div><div class="tabs">${book.sheets.map((s,i)=>`<button data-tab="${i}" class="${i===sheetIndex?'current':''}">${escapeHtml(s.name)}</button>`).join('')}</div></div>`;
   $('details').textContent=`${book.sheets.length} sheets · ${Object.keys(b.cells).length} filled cells`;
   $('sheet-go-to').onclick=()=>{
     const target=window.prompt('Go to cell (A1 ... ZZ10000)',cols[0]+gridViewport.row);
     if(!target)return;
     try{
       const normalized=target.trim().toUpperCase();
       parseCellAddress(normalized);
       gridViewport=viewportAround(normalized,gridViewport);refresh();
       document.querySelector<HTMLInputElement>('[data-address="'+normalized+'"]')?.focus();
     }catch(error){message(String(error));}
   };
   for(const [button,dr,dc] of [
     ['sheet-prev-cols',0,-gridViewport.cols],['sheet-next-cols',0,gridViewport.cols],
     ['sheet-prev-rows',-gridViewport.rows,0],['sheet-next-rows',gridViewport.rows,0],
   ] as const){$(button).onclick=()=>{gridViewport=nextViewport(gridViewport,dr,dc);refresh();};}
   $('sheet-sort').onclick=()=>void run(async()=>{
     const raw=window.prompt('Sort rectangle (up to 2,000 rows and 30 columns)','A1:D20');
     if(!raw)return;
     const range=parseRange(raw);
     const keyColumn=window.prompt('Sort by column (e.g. B)','B')?.trim().toUpperCase();
     if(!keyColumn)return;
     const ascending=window.confirm('Sort ascending? OK = ascending; Cancel = descending');
     const header=window.confirm('Keep first row as headers?');
     if(!window.confirm('Sort '+range+' by '+keyColumn+'? Entire rows within the range will move together. Formulas inside the range are protected.'))return;
     book=await invoke<Book>('workbook_sort_range',{expectedRevision:book.revision,sheet:sheetIndex,range,keyColumn,ascending,header});
     syncDirty();refresh();message('Sorted '+range+'. Save the workbook to retain changes.');
   });
   $('sheet-fill').onclick=()=>void run(async()=>{
     const raw=window.prompt('Fill a row or column (e.g. A1:A100)','A1:A20');
     if(!raw)return;
     const range=parseRange(raw);
     const startText=window.prompt('Starting number','1');if(startText===null)return;
     const stepText=window.prompt('Step between cells','1');if(stepText===null)return;
     const start=Number(startText),step=Number(stepText);
     if(!Number.isFinite(start)||!Number.isFinite(step))throw Error('Series requires finite numbers');
     if(!window.confirm('Replace '+range+' with a numerical series?'))return;
     book=await invoke<Book>('workbook_fill_series',{expectedRevision:book.revision,sheet:sheetIndex,range,start,step});
     syncDirty();refresh();message('Series filled: '+range);
   });
   $('sheet-clear').onclick=()=>void run(async()=>{
     const raw=window.prompt('Clear contents within rectangle','A1:D20');
     if(!raw)return;
     const range=parseRange(raw);
     if(!window.confirm('Clear contents of '+range+'? This changes the workbook.'))return;
     book=await invoke<Book>('workbook_clear_range',{expectedRevision:book.revision,sheet:sheetIndex,range});
     syncDirty();refresh();message('Cleared '+range);
   });
   document.querySelectorAll<HTMLInputElement>('[data-address]').forEach(input=>{input.onpaste=e=>{const raw=e.clipboardData?.getData('text/plain');if(!raw || !(/[\t\n\r]/.test(raw)))return;e.preventDefault();const start=input.dataset.address!;const m=/^([A-Z]+)([0-9]+)$/.exec(start)!;const baseRow=Number(m[2]),baseCol=[...m[1]].reduce((n,c)=>n*26+c.charCodeAt(0)-64,0);const updates:Record<string,string>={};const rows=raw.replace(/\r\n/g,'\n').replace(/\r/g,'\n').replace(/\n$/,'').split('\n');rows.forEach((row,r)=>row.split('\t').forEach((value,c)=>{if(baseRow+r<=10000&&baseCol+c<=702)updates[toAddress(baseRow+r,baseCol+c)]=value;}));void run(async()=>{book=await invoke('workbook_put_batch',{expectedRevision:book.revision,sheet:sheetIndex,updates});syncDirty();refresh();message(`Pasted ${Object.keys(updates).length} cells`);});};input.onchange=()=>void run(async()=>{const address=input.dataset.address!,value=input.value;book=await invoke('workbook_put',{expectedRevision:book.revision,sheet:sheetIndex,address,value});syncDirty();$('rev').textContent=`Revision ${book.revision}`;if(value.startsWith('=')){try{const result=await invoke<string>('workbook_value',{sheet:sheetIndex,address});input.title=`Result: ${result}`;message(`${address} = ${result}`);}catch(e){input.title=String(e);message(String(e));}}});});
   document.querySelectorAll<HTMLButtonElement>('[data-tab]').forEach(b=>b.onclick=()=>{sheetIndex=Number(b.dataset.tab);refresh();});
   $('rename-sheet').onclick=()=>void run(async()=>{const name=window.prompt('Sheet name',book.sheets[sheetIndex].name);if(name===null)return;book=await invoke('workbook_rename_sheet',{expectedRevision:book.revision,index:sheetIndex,name});syncDirty();refresh();});
   $('delete-sheet').onclick=()=>void run(async()=>{if(!window.confirm('Delete this sheet and all its cells?'))return;book=await invoke('workbook_delete_sheet',{expectedRevision:book.revision,index:sheetIndex});sheetIndex=Math.min(sheetIndex,book.sheets.length-1);syncDirty();refresh();});
   $('find-cells').onclick=()=>void run(async()=>{
     const needle=window.prompt('Search across all workbook sheets');if(!needle)return;
     const matches=await invoke<{sheet:number;address:string;value:string}[]>('workbook_find',{needle});
     if(!matches.length){message('No matches found');return;}
     const hit=matches[0];sheetIndex=hit.sheet;refresh();message(`${matches.length} match(es). First: ${book.sheets[hit.sheet].name}!${hit.address}: ${hit.value.slice(0,60)}`);
     const cell=document.querySelector<HTMLInputElement>(`[data-address="${hit.address}"]`);cell?.focus();
   });
   $('sheet-stats').onclick=()=>void run(async()=>{
     const x=await invoke<{populated:number;numeric:number;formulas:number;sum:number;average:number|null;minimum:number|null;maximum:number|null}>('workbook_statistics',{sheet:sheetIndex});
     window.alert(`Sheet: ${book.sheets[sheetIndex].name}\nFilled: ${x.populated}\nNumeric: ${x.numeric}\nFormulas: ${x.formulas}\nSum: ${x.sum}\nAverage: ${x.average??'—'}\nMin: ${x.minimum??'—'}\nMax: ${x.maximum??'—'}`);
   });
   $('import-csv').onclick=()=>($('csv-picker') as HTMLInputElement).click();
   ($('csv-picker') as HTMLInputElement).onchange=()=>void run(async()=>{const file=($('csv-picker') as HTMLInputElement).files?.[0];if(!file)return;if(file.size>5*1024*1024)throw Error('CSV exceeds 5 MiB limit');const records=parseCsv((await file.text()).replace(/^\uFEFF/,''));const updates:Record<string,string>={};records.forEach((row,r)=>row.forEach((value,c)=>{if(r<10000&&c<702)updates[toAddress(r+1,c+1)]=value;}));book=await invoke('workbook_put_batch',{expectedRevision:book.revision,sheet:sheetIndex,updates});syncDirty();refresh();message(`Imported ${Object.keys(updates).length} CSV cells`);});
   $('add-sheet').onclick=()=>void run(async()=>{const name=window.prompt('Name of new sheet',`Sheet ${book.sheets.length+1}`);if(!name)return;book=await invoke('workbook_add_sheet',{expectedRevision:book.revision,name});sheetIndex=book.sheets.length-1;syncDirty();refresh();});
 }else{
   const item=deck.slides[slideIndex];$('canvas').innerHTML=`<div class="slides-area"><div class="slide-list"><div class="slides-title">SLIDES <button id="add-slide">＋</button></div>${deck.slides.map((s,i)=>`<button class="slide-thumb ${i===slideIndex?'current':''}" data-slide="${i}"><span>${i+1}</span><strong>${escapeHtml(s.title||'Untitled slide')}</strong></button>`).join('')}</div><div class="slide-edit"><div class="slide-tools"><span>Slide ${slideIndex+1} of ${deck.slides.length}</span><button id="slide-up">↑</button><button id="slide-down">↓</button><button id="duplicate-slide">Duplicate</button><button id="delete-slide">Delete slide</button><button id="slide-insert-sheet">Insert Sheets table</button><button id="presentation">Present</button>${item.base_source?'<button id="slide-check-base">Check Base</button><button id="slide-refresh-base">Refresh Base</button><button id="slide-reconnect-base">Relink Base</button><button id="slide-detach-base">Detach Base</button>':''}</div><div class="slide-screen"><input id="slide-title" placeholder="Slide title" value="${escapeHtml(item.title)}" aria-label="Slide title"><textarea id="slide-body" aria-label="Slide content" placeholder="Your story begins here..."></textarea><div class="slide-placed-objects">${renderSlideFrames(item.embedded_objects??[],item.object_frames??{},objects=>slideFusion.preview(objects))}</div></div><label class="notes-label">Speaker notes<textarea id="slide-notes" placeholder="Private notes for the presenter..."></textarea></label></div></div>`;
   ($('slide-body') as HTMLTextAreaElement).value=item.body;($('slide-notes') as HTMLTextAreaElement).value=item.notes;
   $('details').textContent=`${deck.slides.length} slides · Presentation editor`;
   let timer:ReturnType<typeof setTimeout>|undefined;
   for(const id of ['slide-title','slide-body','slide-notes'])$(id).addEventListener('input',()=>{syncDirty();clearTimeout(timer);timer=setTimeout(()=>void commitSlide(),350);});
   document.querySelectorAll<HTMLButtonElement>('[data-slide]').forEach(x=>x.onclick=()=>void run(async()=>{await commitSlide();slideIndex=Number(x.dataset.slide);refresh();}));
   slideFusion.wirePreview();
   const slideStage=document.querySelector<HTMLElement>('.slide-screen');
   if(slideStage)wireSlideFrames(slideStage,item.object_frames??{},async(objectId,frame)=>{
     await run(async()=>{
       await commitSlide();
       deck=await invoke<Deck>('slides_embedded_frame',{expectedRevision:deck.revision,index:slideIndex,objectId,frame});
       syncDirty();refresh();
     });
   });
   $('slide-insert-sheet').onclick=()=>void run(async()=>{
     await flush();
     const isCurrent=window.confirm('Copy the active worksheet from Sheets? Cancel inserts a blank table.');
     const label=window.prompt('Embedded Sheets table name',isCurrent?book.sheets[sheetIndex]?.name??'Embedded table':'Embedded table');
     if(!label)return;
     deck=await invoke<Deck>('slides_embed_sheet',{expectedRevision:deck.revision,index:slideIndex,label,sourceSheet:isCurrent?sheetIndex:null});
     syncDirty();refresh();message('Editable Sheets table inserted in Slides. Save the presentation.');
   });
   $('add-slide').onclick=()=>void run(async()=>{await commitSlide();deck=await invoke('deck_add',{expectedRevision:deck.revision});slideIndex=deck.slides.length-1;syncDirty();refresh();});
   $('duplicate-slide').onclick=()=>void run(async()=>{await commitSlide();deck=await invoke('deck_duplicate',{expectedRevision:deck.revision,index:slideIndex});slideIndex++;syncDirty();refresh();});
   for(const [button,direction] of [['slide-up',-1],['slide-down',1]] as const){$(button).onclick=()=>void run(async()=>{await commitSlide();const next=slideIndex+direction;if(next<0||next>=deck.slides.length)return;deck=await invoke('deck_move',{expectedRevision:deck.revision,from:slideIndex,to:next});slideIndex=next;syncDirty();refresh();});}
   $('delete-slide').onclick=()=>void run(async()=>{if(!window.confirm('Delete this slide?'))return;deck=await invoke('deck_delete',{expectedRevision:deck.revision,index:slideIndex});slideIndex=Math.min(slideIndex,deck.slides.length-1);syncDirty();refresh();});
   if(item.base_source){
     $('slide-check-base').onclick=()=>void run(async()=>{
       await commitSlide();
       const link=deck.slides[slideIndex]?.base_source;
       if(!link)return;
       if(!window.confirm('Read linked Base source?\n'+link.database_path+'\nTable: '+link.table))return;
       const status=await invoke<{state:string;detail:string;differing_fields:string[]}>('slides_base_link_status',{index:slideIndex});
       message('Base link '+status.state+': '+status.detail+(status.differing_fields.length?' Fields: '+status.differing_fields.join(', '):''));
     });
     $('slide-refresh-base').onclick=()=>void run(async()=>{
       await commitSlide();
       const link=deck.slides[slideIndex]?.base_source;
       if(!link)return;
       if(!window.confirm('Refresh this slide from the linked database?\n'+link.database_path+'\nTable: '+link.table+'\nLocal slide edits will block refresh.'))return;
       const status=await invoke<{state:string;detail:string}>('slides_base_link_status',{index:slideIndex});
       if(status.state!=='changed'){message('Base link '+status.state+': '+status.detail);return;}
       deck=await invoke<Deck>('slides_base_link_refresh',{index:slideIndex,expectedRevision:deck.revision});
       syncDirty();refresh();message('Slide updated from Base. Save presentation to persist.');
     });
     $('slide-reconnect-base').onclick=()=>void run(async()=>{
       await commitSlide();
       const link=deck.slides[slideIndex]?.base_source;
       if(!link)return;
       const selected=await open({multiple:false,filters:[{name:'Koffice Base database',extensions:['kdb']}]});
       if(typeof selected!=='string')return;
       if(!window.confirm('Reconnect this slide to table '+link.table+' in the selected database?\n'+selected+'\nLocal edits will prevent reconnect.'))return;
       deck=await invoke<Deck>('slides_base_link_reconnect',{index:slideIndex,expectedRevision:deck.revision,path:selected});
       syncDirty();refresh();message('Database source reconnected. Save Slides to persist.');
     });
     $('slide-detach-base').onclick=()=>void run(async()=>{
       await commitSlide();
       if(!window.confirm('Detach this Base link? Current slide contents remain unchanged.'))return;
       deck=await invoke<Deck>('slides_base_link_detach',{index:slideIndex,expectedRevision:deck.revision});
       syncDirty();refresh();message('Base link detached; slide data preserved.');
     });
   }
   $('presentation').onclick=()=>{const data=deck.slides.map(s=>`<section style="page-break-after:always;display:grid;place-content:center;width:100vw;height:100vh;background:#202632;color:white;padding:7vw;box-sizing:border-box;font:28px sans-serif"><h1 style="font-size:2em">${escapeHtml(s.title)}</h1><p style="white-space:pre-wrap">${escapeHtml(s.body)}</p><div style="font-size:13px;max-height:45vh;overflow:auto;background:white;color:#222">${renderStaticSlideFrames(s.embedded_objects??[],s.object_frames??{},objects=>slideFusion.exportHtml(objects))}</div></section>`).join('');const w=window.open('','_blank');if(!w){message('Allow popups to present');return;}w.document.write(`<html><head><title>${escapeHtml(deck.title)}</title></head><body style="margin:0">${data}</body></html>`);w.document.close();};
 }
}
async function commitWriter(){if(active!=='writer')return;const editor=$('text') as HTMLDivElement|null;if(!editor)return;const {text,html}=readRichEditor(editor);if(text===doc.text&&html===(doc.rich_html??plaintextRichHtml(doc.text)))return;doc=await invoke<Doc>('edit_document',{expectedRevision:doc.revision,edit:{type:'replace_rich',text,html}});syncDirty();$('rev').textContent=`Revision ${doc.revision}`;$('details').textContent=`${text.trim()?text.trim().split(/\s+/u).length:0} words · ${[...text].length} characters`;}
async function commitSlide(){if(active!=='slides')return;const title=($('slide-title') as HTMLInputElement)?.value,body=($('slide-body') as HTMLTextAreaElement)?.value,notes=($('slide-notes') as HTMLTextAreaElement)?.value;if(title===undefined)return;const original=deck.slides[slideIndex];const slide:Slide={title,body,notes,...(original.base_source?{base_source:original.base_source}:{}),...(original.embedded_objects?.length?{embedded_objects:original.embedded_objects}:{}),...(original.object_frames?{object_frames:original.object_frames}:{})};if(JSON.stringify(slide)===JSON.stringify(original))return;deck=await invoke('deck_update',{expectedRevision:deck.revision,index:slideIndex,slide});syncDirty();$('rev').textContent=`Revision ${deck.revision}`;}
async function flush(){if(active==='base')return;if(active==='writer')await commitWriter();if(active==='slides')await commitSlide();}
const commands:Record<Kind,{create:string;open:string;save:string}>={writer:{create:'new_document',open:'open_document',save:'save_document'},sheets:{create:'workbook_new',open:'workbook_open',save:'workbook_save'},slides:{create:'deck_new',open:'deck_open',save:'deck_save'},base:{create:'base_create',open:'base_open',save:'base_path'}};
async function create(){if(active==='base'){await base.create();return;}if(!confirmDiscard())return;const next=await invoke(commands[active].create);assign(next);paths[active]=null;saved[active]=revision();dirty=false;sheetIndex=slideIndex=0;refresh();}
function assign(next:unknown){if(active==='writer')doc=next as Doc;else if(active==='sheets')book=next as Book;else deck=next as Deck;}
async function openFile(){if(active==='base'){await base.open();return;}if(!confirmDiscard())return;const selected=await open({multiple:false,filters:[{name:`Koffice ${labels[active]}`,extensions:[extensions[active]]}]});if(typeof selected!=='string')return;assign(await invoke(commands[active].open,{path:selected}));paths[active]=selected;saved[active]=revision();dirty=false;sheetIndex=slideIndex=0;refresh();}
async function saveFile(force:boolean){if(active==='base'){message('Base saves each transaction automatically in SQLite');return;}await flush();let destination=force?null:paths[active];if(!destination)destination=await save({defaultPath:title().replace(/[\\/:*?"<>|]/g,'_')+'.'+extensions[active],filters:[{name:labels[active],extensions:[extensions[active]]}]});if(!destination)return;await invoke(commands[active].save,{path:destination});paths[active]=destination;saved[active]=revision();dirty=false;refresh();message('Saved successfully');}
function exportData(){if(active==='base'){base.exportCsv();return;}if(active==='sheets'){const sheet=book.sheets[sheetIndex];const cells=Object.entries(sheet.cells);const maxRow=Math.min(10000,Math.max(1,...cells.map(([a])=>Number(a.match(/\d+$/)?.[0]||0))));const maxCol=Math.min(702,Math.max(1,...cells.map(([a])=>{const v=a.match(/^[A-Z]+/)?.[0]||'A';return [...v].reduce((n,c)=>n*26+c.charCodeAt(0)-64,0);})));if(maxRow*maxCol>200000){message('CSV export exceeds 200,000 cells; select a smaller sheet');return;}const rows=Array.from({length:maxRow},(_,r)=>Array.from({length:maxCol},(_,c)=>{const key=toAddress(r+1,c+1);const v=sheet.cells[key]||'';return '"'+v.replace(/"/g,'""')+'"';}).join(','));download(new Blob(['\ufeff'+rows.join('\r\n')],{type:'text/csv;charset=utf-8'}),sheet.name+'.csv');return;}
 if(active==='writer'){
   if(window.confirm('Export Writer as HTML? (Cancel exports plain TXT)')){
     const html=`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escapeHtml(doc.title)}</title><style>body{max-width:800px;margin:4rem auto;padding:0 2rem;font:18px/1.7 system-ui;color:#222}article{white-space:pre-wrap;overflow-wrap:anywhere}${cssPageStyle(doc.page??DEFAULT_PAGE)}.writer-print-header,.writer-print-footer{font-size:10px;color:#555}.writer-print-body section{break-before:page;page-break-before:always;min-height:0}@media print{body{margin:0}}</style></head><body><h1>${escapeHtml(doc.title)}</h1><article>${pagePrintableHtml(doc.page??DEFAULT_PAGE,sanitizeRichHtml(doc.rich_html??plaintextRichHtml(doc.text)))}</article>${fusion.exportHtml()}</body></html>`;
     download(new Blob([html],{type:'text/html;charset=utf-8'}),doc.title+'.html');
   } else download(new Blob([doc.text],{type:'text/plain;charset=utf-8'}),doc.title+'.txt');
   return;
 }
 if(active==='slides'){
   const content=deck.slides.map(s=>`<section><h1>${escapeHtml(s.title)}</h1><p>${escapeHtml(s.body)}</p>${renderStaticSlideFrames(s.embedded_objects??[],s.object_frames??{},objects=>slideFusion.exportHtml(objects))}</section>`).join('');
   const html=`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${escapeHtml(deck.title)}</title><style>body{margin:0;background:#12161c;color:#f4f5f7;font:26px/1.45 system-ui}section{min-height:100vh;box-sizing:border-box;display:flex;flex-direction:column;justify-content:center;padding:9vw;border-bottom:2px solid #333}h1{font-size:2em}p{white-space:pre-wrap}@media print{section{page-break-after:always;border:0}}</style></head><body>${content}</body></html>`;
   download(new Blob([html],{type:'text/html;charset=utf-8'}),deck.title+'.html');return;
 }
}
function download(blob:Blob,name:string){const url=URL.createObjectURL(blob);const a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
function printFile(){if(active==='base'){message('Print is not implemented for Base yet');return;}if(active==='sheets'){message('Use Export to download a CSV file');return;}const w=window.open('','_blank');if(!w){message('Allow popups to print');return;}const content=active==='writer'?`<h1>${escapeHtml(doc.title)}</h1>${pagePrintableHtml(doc.page??DEFAULT_PAGE,sanitizeRichHtml(doc.rich_html??plaintextRichHtml(doc.text)))}${fusion.exportHtml()}`:deck.slides.map(s=>`<section style="page-break-after:always"><h1>${escapeHtml(s.title)}</h1><div style="white-space:pre-wrap">${escapeHtml(s.body)}</div>${renderStaticSlideFrames(s.embedded_objects??[],s.object_frames??{},objects=>slideFusion.exportHtml(objects))}</section>`).join('');w.document.write(`<html><head><title>${escapeHtml(title())}</title><style>body{font:16px/1.6 Arial,sans-serif;padding:42px}section{min-height:80vh}${active==='writer'?cssPageStyle(doc.page??DEFAULT_PAGE):''}.writer-print-header,.writer-print-footer{font-size:10px;color:#555}.writer-print-body section{break-before:page;page-break-before:always;min-height:0}@media print{body{padding:0}}</style></head><body>${content}</body></html>`);w.document.close();w.focus();w.print();}
for(const kind of ['writer','sheets','slides','base'] as const)$(`nav-${kind}`).onclick=()=>void run(async()=>{if(desktopMode!=='suite')return;await flush();active=kind;dirty=saved[active]!==revision();refresh();});
function findReplace(){if(active!=='writer'){message('Find / Replace is available in Writer');return;}const needle=window.prompt('Find text',searchNeedle);if(needle===null||!needle)return;searchNeedle=needle;const el=$('text') as HTMLDivElement;if(!replaceEditorMatch(el,needle,needle)){message('No matches');return;}if(!window.confirm('Replace this match? (Cancel = find only)'))return;const replacement=window.prompt('Replace with',needle);if(replacement===null)return;replaceEditorMatch(el,needle,replacement);}
// The automation console is opt-in and local-only. It does not expose a network port.
const consoleRoot=document.createElement('div');
consoleRoot.id='automation-console';consoleRoot.className='automation-overlay';consoleRoot.hidden=true;
consoleRoot.innerHTML=`<div class="automation-panel" role="dialog" aria-modal="true" aria-label="Koffice Automation API"><div class="automation-head"><strong>Koffice · Automation API</strong><button id="automation-close">Close</button></div><p>Typed JSON operations run locally in Rust. Up to 50 commands form one atomic Writer / Sheets / Slides transaction. No network access.</p><textarea id="automation-input" spellcheck="false" aria-label="Automation JSON"></textarea><div class="automation-actions"><button id="automation-snapshot">Refresh snapshot</button><button id="automation-run">Execute batch</button></div><pre id="automation-output" aria-live="polite"></pre></div>`;
document.body.append(consoleRoot);
const automationInput=()=>$('automation-input') as HTMLTextAreaElement;
const automationOutput=()=>$('automation-output');
const example=()=>({operations:[{type:'sheets_put_batch',expected_revision:book.revision,sheet:sheetIndex,updates:{A1:'Koffice',B1:'AI-ready'}}]});
async function openAutomation(){await flush();consoleRoot.hidden=false;automationInput().value=JSON.stringify(example(),null,2);const state=await invoke('automation_snapshot');automationOutput().textContent=JSON.stringify(state,null,2).slice(0,15000);automationInput().focus();}
$('fusion-center').onclick=()=>void run(async()=>{await flush();await fusionManager.open();});
$('automation').onclick=()=>void run(openAutomation);
$('automation-close').onclick=()=>{consoleRoot.hidden=true;};
$('automation-snapshot').onclick=()=>void run(async()=>{automationOutput().textContent=JSON.stringify(await invoke('automation_snapshot'),null,2).slice(0,15000);});
$('automation-run').onclick=()=>void run(async()=>{
  await flush();const raw=automationInput().value;if(raw.length>1024*1024)throw Error('Automation request exceeds 1 MiB');
  const batch=JSON.parse(raw) as {operations:unknown[]};if(!batch||!Array.isArray(batch.operations))throw Error('Expected JSON object with operations array');
  if(!window.confirm(`Apply ${batch.operations.length} operation(s) to currently open workspaces?`))return;
  const result=await invoke('automation_execute',{batch});
  const [w,b,d]=await Promise.all([invoke<Doc>('snapshot'),invoke<Book>('workbook_snapshot'),invoke<Deck>('deck_snapshot')]);
  doc=w;book=b;deck=d;sheetIndex=Math.min(sheetIndex,book.sheets.length-1);slideIndex=Math.min(slideIndex,deck.slides.length-1);
  dirty=saved[active]!==revision();refresh();automationOutput().textContent=JSON.stringify(result,null,2);message('Automation transaction committed');
});
$('find').onclick=()=>findReplace();
$('new').onclick=()=>void run(create);$('open').onclick=()=>void run(openFile);$('save').onclick=()=>void run(()=>saveFile(false));$('saveas').onclick=()=>void run(()=>saveFile(true));$('export').onclick=exportData;$('print').onclick=printFile;
function changeZoom(delta:number){preferences.scale=Math.max(.8,Math.min(1.35,Math.round((preferences.scale+delta)*20)/20));applyPreferences();}
$('toggle-compact').onclick=()=>{preferences.compact=!preferences.compact;applyPreferences();};
$('zoom-out').onclick=()=>changeZoom(-.05);
$('zoom-in').onclick=()=>changeZoom(.05);
applyPreferences();
window.addEventListener('keydown',e=>{
 if(!(e.ctrlKey||e.metaKey)||e.altKey)return;
 const key=e.key.toLowerCase();
 if(key==='s'){e.preventDefault();void run(()=>saveFile(e.shiftKey));return;}
 if(key==='o'){e.preventDefault();void run(openFile);return;}
 if(key==='n'){e.preventDefault();void run(create);return;}
 if(key==='b'&&e.shiftKey){e.preventDefault();preferences.compact=!preferences.compact;applyPreferences();return;}
 if(key==='0'&&e.shiftKey){e.preventDefault();preferences.scale=1;applyPreferences();return;}
 if(desktopMode==='suite'&&!e.shiftKey&&['1','2','3','4'].includes(key)){
   e.preventDefault();const kind=(['writer','sheets','slides','base'] as const)[Number(key)-1];
   void run(async()=>{await flush();active=kind;dirty=saved[active]!==revision();refresh();});
 }
});
window.addEventListener('beforeunload',e=>{if(dirty||saved.writer!==doc.revision||saved.sheets!==book.revision||saved.slides!==deck.revision)e.preventDefault();});
void Promise.all([invoke<Doc>('snapshot'),invoke<Book>('workbook_snapshot'),invoke<Deck>('deck_snapshot'),invoke<DesktopMode>('desktop_mode')]).then(([d,b,p,mode])=>{doc=d;book=b;deck=p;saved.writer=d.revision;saved.sheets=b.revision;saved.slides=p.revision;applyDesktopMode(mode);refresh();void base.initialize().catch(e=>message(String(e)));if(mode==='suite')showAppLauncher();}).catch(e=>message(`Koffice Core unavailable: ${e}`));