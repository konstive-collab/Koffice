import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
const src=readFileSync('src/sheets-style.ts','utf8');
const helper=readFileSync('src/sheets-workbench.ts','utf8');
const ui=readFileSync('src/main.ts','utf8');
const native=readFileSync('src-tauri/src/suite.rs','utf8');
const bridge=readFileSync('src-tauri/src/lib.rs','utf8');
const transpile=c=>ts.transpileModule(c,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const helperUrl='data:text/javascript;base64,'+Buffer.from(transpile(helper)).toString('base64');
const js=transpile(src).replaceAll("'./sheets-workbench'","'"+helperUrl+"'");
const style=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));
test('style classes use a fixed safe palette',()=>{
 assert.equal(style.styleClass({bold:true,italic:true,align:'center',fill:'yellow'}),'s-bold s-italic s-align-center s-fill-yellow');
 assert.throws(()=>style.validateStyle({bold:true,italic:false,align:'left',fill:'url(javascript:alert(1))'}),/Unsupported/);
});
test('style updates cover full selected area and enforce limit',()=>{
 const updates=style.styleUpdates('B2:C3',{bold:true,italic:false,align:'right',fill:'green'});
 assert.deepEqual(Object.keys(updates).sort(),['B2','B3','C2','C3']);
 assert.throws(()=>style.styleUpdates('A1:ZZ100',style.EMPTY_STYLE),/5,000/);
});
test('frozen rows and columns stay visible while navigating',()=>{
 assert.deepEqual(style.gridRows(120,5,2),[1,2,120,121,122]);
 assert.deepEqual(style.gridRows(1,4,1),[1,2,3,4]);
 assert.deepEqual(style.gridRows(350,4,0),[350,351,352,353]);
});
test('native model persists cell styles and frozen panes',()=>{
 assert.match(native,/pub styles:BTreeMap<String,CellStyle>/);
 assert.match(native,/pub freeze:Option<SheetFreeze>/);
 assert.match(native,/cell_style_and_freeze_roundtrip_and_legacy/);
 assert.match(native,/invalid_style_and_freeze_do_not_mutate_book/);
 assert.match(native,/pub fn style_cells/);
 assert.match(native,/pub fn set_freeze/);
});
test('UI uses saved formatting and connects native operations',()=>{
 for(const command of ['workbook_style_cells','workbook_set_freeze']){
  assert.ok(bridge.includes('fn '+command+'('));
  assert.ok(ui.includes("'"+command+"'"));
 }
 assert.match(ui,/id="sheet-style"/);
 assert.match(ui,/id="sheet-freeze"/);
 assert.match(ui,/styleClass\(b.styles\?\.\[c\+row\]\)/);
 assert.match(ui,/gridRows\(gridViewport.row,gridViewport.rows,freeze.rows\)/);
});
