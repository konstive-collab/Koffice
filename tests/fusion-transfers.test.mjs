import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const core=readFileSync('src-tauri/src/fusion_transfer.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/fusion-ui.ts','utf8');
const main=readFileSync('src/main.ts','utf8');
const base=readFileSync('src/base-ui.ts','utf8');

test('embedded sheet extraction copies full worksheet and preserves existing book',()=>{
  assert.match(core,/pub fn append_embedded_to_workbook/);
  assert.match(core,/candidate\.sheets\.push\(sheet\)/);
  assert.match(core,/sheet\.name = name/);
  assert.match(core,/candidate\.validate\(\)\?/);
  assert.match(core,/embedded_sheet_transfer_preserves_formula_and_other_sheets/);
});
test('Base snapshot is explicitly static, capped and has rollback tests',()=>{
  assert.match(core,/pub fn append_base_snapshot/);
  assert.match(core,/const COLS: usize = 8/);
  assert.match(core,/const ROWS: usize = 20/);
  assert.match(core,/No live database link/);
  assert.match(core,/Snapshot truncated/);
  assert.match(core,/database_snapshot_is_bounded_and_clear_about_truncation/);
});
test('both Rust Tauri transfer commands are wired',()=>{
  assert.match(lib,/fn writer_embedded_to_sheets/);
  assert.match(lib,/fn base_snapshot_to_slides/);
  assert.match(lib,/fusion_transfer::append_embedded_to_workbook/);
  assert.match(lib,/fusion_transfer::append_base_snapshot/);
});
test('Writer embedded table exposes export to Sheets',()=>{
  assert.match(ui,/data-fusion-extract/);
  assert.match(ui,/this\.extract/);
  assert.match(main,/writer_embedded_to_sheets/);
});
test('Base UI provides a slide snapshot action',()=>{
  assert.match(base,/id="base-to-slides"/);
  assert.match(base,/this\.toSlides/);
  assert.match(main,/base_snapshot_to_slides/);
});
