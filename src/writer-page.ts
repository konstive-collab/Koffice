/** Physical page configuration for Writer, stored in native .kdoc. */
export type WriterPage={paper:'a4'|'letter'|'legal';landscape:boolean; margin_top_mm:number;margin_right_mm:number;margin_bottom_mm:number;margin_left_mm:number;header:string;footer:string};
export const DEFAULT_PAGE:WriterPage={paper:'a4',landscape:false,margin_top_mm:20,margin_right_mm:20,margin_bottom_mm:20,margin_left_mm:20,header:'',footer:''};
export function paperDimensions(page:WriterPage):{width:number;height:number}{
 const size=page.paper==='letter'?[216,279]:page.paper==='legal'?[216,356]:[210,297];
 return page.landscape?{width:size[1],height:size[0]}:{width:size[0],height:size[1]};
}
export function pageCss(page:WriterPage):Record<string,string>{
 const size=paperDimensions(page);
 return {'--writer-paper-width':size.width+'mm','--writer-paper-height':size.height+'mm',
 '--writer-margin-top':page.margin_top_mm+'mm','--writer-margin-right':page.margin_right_mm+'mm',
 '--writer-margin-bottom':page.margin_bottom_mm+'mm','--writer-margin-left':page.margin_left_mm+'mm'};
}
export function cssPageStyle(page:WriterPage):string{
 const d=paperDimensions(page);
 return '@page{size:'+d.width+'mm '+d.height+'mm;margin:'+page.margin_top_mm+'mm '+page.margin_right_mm+'mm '+page.margin_bottom_mm+'mm '+page.margin_left_mm+'mm}';
}
export function escapePageText(value:string):string {
 return value.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
}
export function pagePrintableHtml(page:WriterPage,bodyHtml:string):string{
 return '<div class="writer-print-layout"><header class="writer-print-header">'+escapePageText(page.header)+'</header>'+
 '<main class="writer-print-body">'+bodyHtml+'</main><footer class="writer-print-footer">'+escapePageText(page.footer)+'</footer></div>';
}
export function openPageSetup(current:WriterPage,onApply:(page:WriterPage)=>Promise<void>):void{
 const overlay=document.createElement('div');
 overlay.className='writer-page-dialog';
 overlay.innerHTML='<form class="writer-page-form" aria-label="Writer page setup">'+
 '<h2>Page setup</h2><p>Paper size, margins and document headers are saved in this .kdoc.</p>'+
 '<div class="writer-page-fields">'+
 '<label>Paper<select name="paper"><option value="a4">A4 · 210 × 297 mm</option><option value="letter">Letter · 216 × 279 mm</option><option value="legal">Legal · 216 × 356 mm</option></select></label>'+
 '<label>Orientation<select name="landscape"><option value="false">Portrait</option><option value="true">Landscape</option></select></label>'+
 '<label>Top margin (mm)<input name="margin_top_mm" type="number" min="5" max="60" step="1" required></label>'+
 '<label>Right margin (mm)<input name="margin_right_mm" type="number" min="5" max="60" step="1" required></label>'+
 '<label>Bottom margin (mm)<input name="margin_bottom_mm" type="number" min="5" max="60" step="1" required></label>'+
 '<label>Left margin (mm)<input name="margin_left_mm" type="number" min="5" max="60" step="1" required></label>'+
 '<label class="writer-page-wide">Header text<input name="header" maxlength="300" placeholder="Optional document header"></label>'+
 '<label class="writer-page-wide">Footer text<input name="footer" maxlength="300" placeholder="Optional document footer"></label>'+
 '</div><div class="writer-page-actions"><button type="button" data-cancel>Cancel</button><button type="submit">Apply</button></div></form>';
 document.body.append(overlay);
 const form=overlay.querySelector<HTMLFormElement>('form')!;
 const field=(name:string)=>form.elements.namedItem(name) as HTMLInputElement|HTMLSelectElement;
 for(const key of ['paper','margin_top_mm','margin_right_mm','margin_bottom_mm','margin_left_mm','header','footer'] as const)field(key).value=String(current[key]);
 field('landscape').value=String(current.landscape);
 const close=()=>overlay.remove();
 overlay.querySelector<HTMLButtonElement>('[data-cancel]')!.onclick=close;
 overlay.onclick=e=>{if(e.target===overlay)close();};
 form.onsubmit=e=>{
   e.preventDefault();
   const next:WriterPage={
    paper:field('paper').value as WriterPage['paper'],landscape:field('landscape').value==='true',
    margin_top_mm:Number(field('margin_top_mm').value),margin_right_mm:Number(field('margin_right_mm').value),
    margin_bottom_mm:Number(field('margin_bottom_mm').value),margin_left_mm:Number(field('margin_left_mm').value),
    header:field('header').value,footer:field('footer').value
   };
   const d=paperDimensions(next);
   if([next.margin_top_mm,next.margin_right_mm,next.margin_bottom_mm,next.margin_left_mm].some(v=>!Number.isInteger(v)||v<5||v>60)||
    next.margin_left_mm+next.margin_right_mm+40>=d.width||next.margin_top_mm+next.margin_bottom_mm+40>=d.height){
    window.alert('Invalid page margins or insufficient printable area.');return;
   }
   void onApply(next).then(close).catch(error=>window.alert(String(error)));
 };
 field('paper').focus();
}
