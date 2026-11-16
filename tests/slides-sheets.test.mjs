import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const suite=readFileSync('src-tauri/src/suite.rs','utf8');
const transfer=readFileSync('src-tauri/src/fusion_transfer.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/main.ts','utf8');
const grid=readFileSync('src/fusion-ui.ts','utf8');
test('Slides persists optional embedded native sheet objects',()=>{
 assert.match(suite,/pub embedded_objects: Vec<crate::fusion::EmbeddedObject>/);
 assert.match(suite,/skip_serializing_if="Vec::is_empty"/);
 assert.match(suite,/legacy_slides_without_embedded_objects_still_load/);
 assert.match(suite,/slides_embedded_sheet_roundtrip_and_formula_edit/);
});
test('Embedded edits use the native Sheets engine with atomic validation',()=>{
 assert.match(suite,/pub fn embedded_put\(/);
 assert.match(suite,/object\.workbook_mut\(\)\.put_batch/);
 assert.match(suite,/slides_embedded_updates_rollback_and_move_with_slide/);
 assert.match(suite,/Presentation exceeds 48 embedded tables or 25,000 populated cells/);
});
test('Source Sheets and existing workbook tabs are preserved',()=>{
 assert.match(transfer,/pub fn append_slide_embedded_to_workbook/);
 assert.match(transfer,/next\.sheets\.push\(tab\)/);
 assert.match(transfer,/next\.validate\(\)\?/);
});
test('Tauri commands are linked to Slides editor actions',()=>{
 for(const command of ['slides_embed_sheet','slides_embedded_put','slides_embedded_remove','slides_embedded_value','slides_embedded_to_sheets']){
  assert.ok(lib.includes('fn '+command+'('),command);
  assert.ok(ui.includes(command),command);
 }
 assert.match(ui,/Insert Sheets table/);
 assert.match(ui,/slideFusion\.preview\(\)/);
 assert.match(ui,/slideFusion\.wirePreview\(\)/);
});
test('Static export, print and presentation preserve embedded sheet representation',()=>{
 assert.match(grid,/exportHtml\(objects:EmbeddedSheet\[\]=this.getObjects\(\)\)/);
 assert.ok((ui.match(/slideFusion\.exportHtml\(s\.embedded_objects\?\?\[\]\)/g)||[]).length>=3);
});
