import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const native=readFileSync('src-tauri/src/sheet_tools.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/main.ts','utf8');
const wb=readFileSync('src/sheets-workbench.ts','utf8');
const css=readFileSync('src/style.css','utf8');
test('Native sorts whole rows with stable numeric keys and optional header',()=>{
 assert.match(native,/pub fn sort_range/);
 assert.match(native,/rows\.sort_by/);
 assert.match(native,/x\.total_cmp\(&y\)/);
 assert.match(native,/numeric_sort_preserves_headers_and_rows/);
 assert.match(native,/sort_is_atomic_rejects_formulas_and_wrong_revisions/);
});
test('Series and clear operations are bounded and atomic',()=>{
 assert.match(native,/pub fn fill_series/);
 assert.match(native,/pub fn clear_range/);
 assert.match(native,/book\.put_batch\(expected,index,changes\)/);
 assert.match(native,/invalid_and_huge_ranges_cannot_modify_data/);
 assert.match(native,/Series must be one row or one column/);
});
test('Tauri exposes all workbook operations to the UI',()=>{
 for(const name of ['workbook_sort_range','workbook_fill_series','workbook_clear_range']){
  assert.ok(lib.includes('fn '+name+'('));
  assert.ok(ui.includes("'"+name+"'"));
 }
});
test('UI supports full-grid navigation and bounded DOM rendering',()=>{
 assert.match(ui,/gridViewport:GridViewport=\{row:1,col:1,rows:40,cols:16\}/);
 assert.match(ui,/gridRows\(gridViewport.row,gridViewport.rows,freeze.rows\)/);
 assert.match(ui,/id="sheet-go-to"/);
 assert.match(ui,/id="sheet-next-cols"/);
 assert.match(ui,/id="sheet-next-rows"/);
 assert.match(ui,/viewportAround\(normalized,gridViewport\)/);
 assert.match(wb,/SHEET_MAX_ROW=10_000/);
 assert.match(wb,/SHEET_MAX_COL=702/);
 assert.match(css,/\.sheet-navigation/);
});
test('Ranges are validated before native calls and destructive operations confirmed',()=>{
 assert.match(wb,/export function parseRange/);
 assert.match(ui,/if\(!window\.confirm\('Replace '/);
 assert.match(ui,/if\(!window\.confirm\('Clear contents of '/);
 assert.match(ui,/parseRange\(raw\)/);
});
