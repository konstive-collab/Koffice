/** Fusion Link Center — paths remain inert until an explicitly approved scan. */
export type LinkNode={
 id:string;label:string;mode:'file'|'session'|'embedded';source:string;worksheet:string;
 embedded_revision:number;status:string;
};
export type PreviewRow={
 id:string;label:string;source_path:string;worksheet:string;
 state:string;changed_cells:number;examples:{address:string;embedded:string;source:string}[];detail:string;
};
export type PreviewReport={token:number;expected_revision:number;rows:PreviewRow[]};
const escape=(v:string)=>v.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export class FusionManagerUI {
 private root:HTMLDivElement;
 private nodes:LinkNode[]=[];
 private selected=new Set<string>();
 private pending:PreviewReport|null=null;
 private busy=false;
 constructor(
  private list:()=>Promise<LinkNode[]>,
  private preview:(ids:string[])=>Promise<PreviewReport>,
  private apply:(report:PreviewReport)=>Promise<void>,
  private notice:(text:string)=>void,
 ){
  this.root=document.createElement('div');
  this.root.className='fusion-manager-overlay';this.root.hidden=true;
  document.body.appendChild(this.root);
  this.root.addEventListener('keydown',e=>{if(e.key==='Escape'){e.stopPropagation();this.close();}});
 }
 private async task(fn:()=>Promise<void>){
  if(this.busy)return;
  this.busy=true;
  try{await fn();}
  catch(e){this.notice(String(e));this.pending=null;this.render();}
  finally{this.busy=false;}
 }
 async open(){
  await this.task(async()=>{
   this.nodes=await this.list();
   this.selected=new Set(this.nodes.filter(n=>n.mode==='file').map(n=>n.id));
   this.pending=null;this.render();
  });
 }
 close(){this.root.hidden=true;this.pending=null;}
 private render(){
  this.root.hidden=false;
  const count=this.nodes.filter(n=>n.mode==='file').length;
  const list=this.nodes.length?this.nodes.map(n=>{
   const allowed=n.mode==='file';
   const checked=this.selected.has(n.id)?'checked':'';
   return '<tr><td><input type="checkbox" data-fm-id="'+escape(n.id)+'" '+checked+(allowed?'':' disabled')+' aria-label="Select '+escape(n.label)+'"></td><td><strong>'+escape(n.label)+'</strong></td><td>'+escape(n.mode)+'</td><td class="fusion-manager-source" title="'+escape(n.source)+'">'+escape(n.source||'—')+'</td><td>'+escape(n.worksheet||'—')+'</td><td>'+escape(n.status)+'</td></tr>';
  }).join(''):'<tr><td colspan="6">No embedded objects in Writer.</td></tr>';
  const rows=this.pending?.rows.map(r=>{
   const details=r.examples.length?'<details><summary>'+r.examples.length+' example changes</summary><ul>'+r.examples.map(x=>'<li><code>'+escape(x.address)+'</code>: '+escape(x.embedded)+' → '+escape(x.source)+'</li>').join('')+'</ul></details>':'';
   return '<div class="fusion-preview-row"><b>'+escape(r.label)+'</b><span>'+escape(r.state)+' · '+r.changed_cells+' cells changed</span><p>'+escape(r.detail)+'</p>'+details+'</div>';
  }).join('')??'';
  const invalid=this.pending?.rows.some(r=>r.state!=='current'&&r.state!=='changed')??false;
  const changed=this.pending?.rows.filter(r=>r.state==='changed').length??0;
  this.root.innerHTML='<section class="fusion-manager-panel" role="dialog" aria-modal="true" aria-label="Fusion Link Center"><header class="fusion-manager-header"><div><h2>Fusion Link Center</h2><p>Inventory, preview and atomic updates of file-backed Sheets tables in Writer.</p></div><button id="fm-close" aria-label="Close">✕</button></header><div class="fusion-manager-body"><p class="fusion-manager-help">Listing never opens external files. File checks happen only when you approve the exact paths below. Session links are managed individually in Writer.</p><div class="fusion-manager-table"><table><thead><tr><th>Update</th><th>Embedded table</th><th>Type</th><th>Source path / name</th><th>Worksheet</th><th>State</th></tr></thead><tbody>'+list+'</tbody></table></div><div class="fusion-manager-actions"><button id="fm-rescan">Reload inventory</button><button id="fm-preview" '+(!count?'disabled':'')+'>Preview selected files</button><span>'+this.selected.size+' file links selected</span></div>'+ (this.pending?'<section class="fusion-manager-results"><h3>Preview · Writer revision '+this.pending.expected_revision+'</h3>'+rows+'<button id="fm-apply" '+(invalid||!changed?'disabled':'')+'>Apply '+changed+' changes atomically</button></section>':'')+'</div></section>';
  this.root.querySelector<HTMLButtonElement>('#fm-close')!.onclick=()=>this.close();
  this.root.querySelector<HTMLButtonElement>('#fm-rescan')!.onclick=()=>void this.task(async()=>{
   this.nodes=await this.list();this.selected=new Set(this.nodes.filter(n=>n.mode==='file').map(n=>n.id));this.pending=null;this.render();
  });
  this.root.querySelectorAll<HTMLInputElement>('[data-fm-id]').forEach(input=>input.onchange=()=>{
   const id=input.dataset.fmId!;
   if(input.checked)this.selected.add(id);else this.selected.delete(id);
   this.pending=null;this.render();
  });
  this.root.querySelector<HTMLButtonElement>('#fm-preview')!.onclick=()=>void this.task(async()=>{
   const nodes=this.nodes.filter(n=>n.mode==='file'&&this.selected.has(n.id));
   if(!nodes.length){this.notice('Select at least one file-backed link');return;}
   const paths=Array.from(new Set(nodes.map(n=>n.source)));
   if(!window.confirm('Read these linked source files to preview changes?\n\n'+paths.join('\n')+'\n\nNo documents will change during preview.'))return;
   this.pending=await this.preview(nodes.map(n=>n.id));this.render();
  });
  const applyButton=this.root.querySelector<HTMLButtonElement>('#fm-apply');
  if(applyButton)applyButton.onclick=()=>void this.task(async()=>{
   const pending=this.pending;
   if(!pending)throw Error('Preview required');
   const paths=Array.from(new Set(pending.rows.map(r=>r.source_path)));
   const count=pending.rows.filter(r=>r.state==='changed').length;
   if(!window.confirm('Apply '+count+' previewed table updates to Writer?\n\n'+paths.join('\n')+'\n\nAny source or document changes since preview will cancel the entire batch.'))return;
   this.pending=null;
   await this.apply(pending);
   this.nodes=await this.list();
   this.render();
   this.notice('Fusion refresh committed. Save Writer to retain the updated tables.');
  });
 }
}
