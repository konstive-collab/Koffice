import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const rust=readFileSync(new URL('../src-tauri/src/base.rs',import.meta.url),'utf8');
const lib=readFileSync(new URL('../src-tauri/src/lib.rs',import.meta.url),'utf8');
const ui=readFileSync(new URL('../src/base-ui.ts',import.meta.url),'utf8');
const main=readFileSync(new URL('../src/main.ts',import.meta.url),'utf8');
test('power tools use the same Rust backend as the UI',()=>{
 for(const name of ['base_import_csv','base_indexes','base_create_index','base_delete_index','base_browse_page']) {
  assert.ok(lib.includes(`fn ${name}(`),`backend ${name}`);
  assert.ok(ui.includes(`'${name}'`),`UI ${name}`);
 }
 assert.match(lib,/fn base_to_workbook\(/);
 assert.match(main,/base_to_workbook/);
});
test('database batch import has transactional rollback and bounded input',()=>{
 assert.match(rust,/let tx=conn.transaction\(/);
 assert.match(rust,/tx.commit\(/);
 assert.match(rust,/MAX_IMPORT_ROWS/);
 assert.match(rust,/MAX_IMPORT_BYTES/);
 assert.match(rust,/csv_import_is_atomic_and_handles_quoted_fields/);
 assert.match(rust,/indexes_and_pagination/);
});
test('webview sanitizes database records and requires confirmation before replacing Sheets',()=>{
 assert.match(ui,/const html = /);
 assert.match(ui,/html\(v\)/);
 assert.match(main,/Replace the unsaved Sheets workbook/);
});