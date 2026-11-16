import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const native=readFileSync('src-tauri/src/sheet_tools.rs','utf8');
const model=readFileSync('src-tauri/src/suite.rs','utf8');
const bridge=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/main.ts','utf8');
test('row sorting moves values and optional cell styles together',()=>{
 assert.match(native,/let styles=&book\.sheets\[index\]\.styles/);
 assert.match(native,/Vec<Option<crate::suite::CellStyle>>/);
 assert.match(native,/tab\.styles\.insert\(address,style\.clone\(\)\)/);
 assert.match(native,/tab\.styles\.remove\(&address\)/);
 assert.match(native,/sort_moves_formats_with_rows_and_clears_missing_style/);
 assert.match(native,/Cannot sort formulas safely/);
});
test('restore validates incoming workbook and revision before commit',()=>{
 assert.match(model,/pub fn restore_snapshot/);
 assert.match(model,/check_revision\(self\.revision,expected\)/);
 assert.match(model,/snapshot\.validate\(\)\?/);
 assert.match(model,/next\.revision=self\.revision\.checked_add\(1\)/);
 assert.match(model,/workbook_restore_snapshot_is_revision_guarded_and_atomic/);
 assert.match(bridge,/fn workbook_restore_snapshot/);
});
test('session undo redo captures successful mutations and restores snapshots',()=>{
 assert.match(ui,/const sheetPast:Book\[\]=\[\]/);
 assert.match(ui,/const sheetFuture:Book\[\]=\[\]/);
 assert.match(ui,/sheetPast\.push\(before\)/);
 assert.match(ui,/if\(sheetPast\.length>20\)sheetPast\.shift\(\)/);
 assert.match(ui,/async function historyStep/);
 assert.match(ui,/invoke<Book>\('workbook_restore_snapshot'/);
 assert.match(ui,/historyStep\('undo'\)/);
 assert.match(ui,/historyStep\('redo'\)/);
});
test('history does not cross new/open workbook sessions',()=>{
 assert.match(ui,/function clearSheetHistory\(\)/);
 assert.ok((ui.match(/if\(active==='sheets'\)clearSheetHistory\(\)/g)||[]).length>=2);
 assert.match(ui,/sheetHistoryEpoch===epoch/);
 assert.match(ui,/historyStep\('undo'\),false/);
 assert.match(ui,/historyStep\('redo'\),false/);
});
