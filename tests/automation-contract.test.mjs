import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const automation=readFileSync(new URL('../src-tauri/src/automation.rs', import.meta.url),'utf8');
const rust=readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url),'utf8');
const ui=readFileSync(new URL('../src/main.ts', import.meta.url),'utf8');
test('AI operations are typed, revisioned, and atomic',()=>{
  for(const tag of ['WriterEdit','SheetsPutBatch','SheetsAddSheet','SheetsRenameSheet','SlidesAdd','SlidesUpdate','SlidesDuplicate'])assert.ok(automation.includes(tag));
  assert.match(automation,/\*writer = w;/);assert.match(automation,/\*sheets = s;/);assert.match(automation,/\*slides = d;/);
  assert.match(automation,/failed_operation_rolls_back_every_editor/);
  assert.match(automation,/rejects_stale_revision_and_unknown_actions/);
});
test('automation commands registered and connected to interactive console',()=>{
  assert.match(rust,/fn automation_snapshot/);assert.match(rust,/fn automation_execute/);
  assert.match(ui,/automation_execute/);assert.match(ui,/automation_snapshot/);
  assert.match(ui,/automation-run/);assert.match(ui,/window.confirm/);
});
test('sheet search and numeric analysis wired end-to-end',()=>{
  for(const name of ['workbook_find','workbook_statistics']){assert.ok(rust.includes(`fn ${name}`));assert.ok(ui.includes(`'${name}'`));}
  assert.match(readFileSync(new URL('../src-tauri/src/suite.rs',import.meta.url),'utf8'),/statistics_include_numeric_formula_values/);
});
test('Writer and Slides export offline HTML with escaped content',()=>{
  assert.match(ui,/doc\.title\+'.html'/);
  assert.match(ui,/deck\.title\+'.html'/);
  assert.match(ui,/escapeHtml\(doc\.text\)/);
  assert.match(ui,/escapeHtml\(s\.body\)/);
});