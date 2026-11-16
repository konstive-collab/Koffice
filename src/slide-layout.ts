import type {EmbeddedSheet} from './fusion-ui';
export type SlideObjectFrame={x:number;y:number;width:number;height:number};
export const STANDARD_FRAME:SlideObjectFrame={x:90,y:300,width:820,height:490};
const percent=(n:number)=>String(n/10)+'%';
const esc=(s:string)=>s.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const clip=(n:number,lo:number,hi:number)=>Math.max(lo,Math.min(hi,Math.round(n)));
export function renderSlideFrames(objects:EmbeddedSheet[],frames:Record<string,SlideObjectFrame>,preview:(objects:EmbeddedSheet[])=>string):string{
 return objects.map(obj=>{
  const f=frames[obj.id]??STANDARD_FRAME;
  const pos='left:'+percent(f.x)+';top:'+percent(f.y)+';width:'+percent(f.width)+';height:'+percent(f.height);
  const id=esc(obj.id),label=esc(obj.label);
  return '<div class="slide-object" data-slide-object="'+id+'" style="'+pos+'">'+
   '<div class="slide-object-handle" data-slide-move="'+id+'" role="button" tabindex="0" aria-label="Move '+label+'" title="Drag to move; arrows move 1%, Shift+arrows move 5%">⠿ '+label+'</div>'+
   '<div class="slide-object-content">'+preview([obj])+'</div>'+
   '<div class="slide-object-resize" data-slide-resize="'+id+'" role="button" tabindex="0" aria-label="Resize '+label+'" title="Drag to resize; arrows adjust size">◢</div></div>';
 }).join('');
}
export function wireSlideFrames(stage:HTMLElement,frames:Record<string,SlideObjectFrame>,save:(id:string,frame:SlideObjectFrame)=>Promise<void>):void{
 const frameFor=(id:string)=>({...frames[id]??STANDARD_FRAME});
 const attach=(selector:string,isResize:boolean)=>{
  stage.querySelectorAll<HTMLElement>(selector).forEach(handle=>{
   const id=handle.getAttribute(isResize?'data-slide-resize':'data-slide-move');
   if(!id)return;
   handle.addEventListener('keydown',e=>{
    if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key))return;
    e.preventDefault();
    const f=frameFor(id),step=e.shiftKey?50:10;
    const sign=e.key==='ArrowRight'||e.key==='ArrowDown'?1:-1;
    const horizontal=e.key==='ArrowLeft'||e.key==='ArrowRight';
    if(isResize){if(horizontal)f.width=clip(f.width+sign*step,150,1000-f.x);else f.height=clip(f.height+sign*step,100,1000-f.y);}
    else {if(horizontal)f.x=clip(f.x+sign*step,0,1000-f.width);else f.y=clip(f.y+sign*step,0,1000-f.height);}
    void save(id,f);
   });
   handle.addEventListener('pointerdown',e=>{
    if(e.button!==0)return;
    e.preventDefault();
    const el=handle.closest<HTMLElement>('.slide-object');
    if(!el)return;
    const start=frameFor(id),bounds=stage.getBoundingClientRect();
    if(bounds.width<=0||bounds.height<=0)return;
    const px=e.clientX,py=e.clientY,pointerId=e.pointerId;
    let changed=false,current={...start};
    handle.setPointerCapture(pointerId);
    const move=(event:PointerEvent)=>{
     if(event.pointerId!==pointerId)return;
     const dx=Math.round((event.clientX-px)/bounds.width*1000);
     const dy=Math.round((event.clientY-py)/bounds.height*1000);
     if(isResize)current={...start,width:clip(start.width+dx,150,1000-start.x),height:clip(start.height+dy,100,1000-start.y)};
     else current={...start,x:clip(start.x+dx,0,1000-start.width),y:clip(start.y+dy,0,1000-start.height)};
     changed=Object.keys(current).some(k=>current[k as keyof SlideObjectFrame]!==start[k as keyof SlideObjectFrame]);
     el.style.left=percent(current.x);el.style.top=percent(current.y);
     el.style.width=percent(current.width);el.style.height=percent(current.height);
    };
    const finish=(event:PointerEvent)=>{
     if(event.pointerId!==pointerId)return;
     handle.removeEventListener('pointermove',move);
     handle.removeEventListener('pointerup',finish);
     handle.removeEventListener('pointercancel',cancel);
     if(changed)void save(id,current);
    };
    const cancel=(event:PointerEvent)=>{
     if(event.pointerId!==pointerId)return;
     handle.removeEventListener('pointermove',move);
     handle.removeEventListener('pointerup',finish);
     handle.removeEventListener('pointercancel',cancel);
     el.style.left=percent(start.x);el.style.top=percent(start.y);
     el.style.width=percent(start.width);el.style.height=percent(start.height);
    };
    handle.addEventListener('pointermove',move);
    handle.addEventListener('pointerup',finish);
    handle.addEventListener('pointercancel',cancel);
   });
  });
 };
 attach('[data-slide-move]',false);
 attach('[data-slide-resize]',true);
}

/** Static print/export layout, same normalized frames without interactive handles. */
export function renderStaticSlideFrames(objects:EmbeddedSheet[],frames:Record<string,SlideObjectFrame>,render:(objects:EmbeddedSheet[])=>string):string{
 if(!objects.length)return '';
 const inner=objects.map(object=>{
  const f=frames[object.id]??STANDARD_FRAME;
  const style='position:absolute;box-sizing:border-box;overflow:auto;background:white;color:#1b241e;border:1px solid #b9c3ba;'+
   'left:'+percent(f.x)+';top:'+percent(f.y)+';width:'+percent(f.width)+';height:'+percent(f.height)+';font-size:10px';
  return '<div style="'+style+'">'+render([object])+'</div>';
 }).join('');
 return '<div style="position:relative;aspect-ratio:16/9;width:100%;margin:12px auto;overflow:hidden;border:1px solid #57665b;background:#202632">'+inner+'</div>';
}