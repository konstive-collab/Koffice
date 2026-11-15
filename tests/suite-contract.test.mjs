import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const src = readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url), 'utf8');
const domain = readFileSync(new URL('../src-tauri/src/suite.rs', import.meta.url), 'utf8');
const ui = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
test('three applications share native Rust commands', () => {
  for (const command of ['workbook_put','workbook_add_sheet','workbook_value','workbook_save','deck_add','deck_update','deck_delete','deck_save','edit_document']) {
    assert.match(src,new RegExp(`fn ${command}\\(`));
    assert.ok(ui.includes(`'${command}'`) || ui.includes(`${command}`), `UI integration missing ${command}`);
  }
});
test('revision and formula protections are present',()=>{
  assert.match(domain,/Circular reference/);
  assert.match(domain,/Revision conflict/);
  assert.match(domain,/Function requires a range/);
  assert.match(domain,/Too many cells/);
});
test('UI includes all editor names and export actions',()=>{
  for (const feature of ['Writer','Sheets','Slides','Export','Print / PDF']) assert.ok(ui.includes(feature));
});

test('batch editing and slide management are exposed to the frontend',()=>{
 for(const c of ['workbook_put_batch','workbook_rename_sheet','workbook_delete_sheet','deck_duplicate','deck_move']){assert.match(src,new RegExp(`fn ${c}\\(`));assert.ok(ui.includes(`'${c}'`),`missing UI call ${c}`);}
 for(const c of ['AVERAGE','COUNT','MIN','MAX'])assert.ok(domain.includes(`"${c}"`));
 assert.match(domain,/batch_is_atomic_and_checks_revision/);
});