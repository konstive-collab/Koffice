import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const engine=readFileSync('src-tauri/src/engine.rs','utf8');
const fusion=readFileSync('src-tauri/src/fusion.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const main=readFileSync('src/main.ts','utf8');
const ui=readFileSync('src/fusion-ui.ts','utf8');
const bin=readFileSync('src-tauri/src/bin/koffice-mcp.rs','utf8');

test('Fusion stores optional typed objects in backward compatible Writer format',()=>{
  assert.match(engine,/embedded_objects:\s*Vec<EmbeddedObject>/);
  assert.match(engine,/serde\(default,\s*skip_serializing_if/);
  assert.match(engine,/fusion::validate_objects/);
  assert.match(engine,/fn embedded_sheet_roundtrip_preserves_formulas_and_legacy_documents/);
});
test('Writer embedded edits use native Sheets operations and revision checks',()=>{
  assert.match(engine,/pub fn embed_sheet/);
  assert.match(engine,/pub fn embedded_put/);
  assert.match(engine,/object\.workbook_mut\(\)\.put_batch/);
  assert.match(engine,/fn embedded_edits_are_atomic_and_do_not_mutate_original_workbook/);
});
test('Fusion uses identifiers and validates one sheet per object',()=>{
  assert.match(fusion,/enum EmbeddedObject/);
  assert.match(fusion,/Duplicate embedded object identifier/);
  assert.match(fusion,/Embedded sheet must contain exactly one worksheet/);
});
test('Tauri binds Writer and standalone Sheets copy without a visual screenshot',()=>{
  for(const command of ['writer_embed_sheet','writer_embedded_put','writer_remove_embedded','writer_embedded_value']){
    assert.match(lib,new RegExp('fn '+command+'\\('));
    assert.ok(lib.includes(command));
  }
  assert.match(lib,/let mut writer=lock\(&s\.writer\)/);
});
test('Writer UI embeds real cell editor and navigation',()=>{
  assert.match(main,/Insert Sheets table/);
  assert.match(main,/fusion\.preview\(\)/);
  assert.match(main,/fusion\.wirePreview\(\)/);
  assert.match(ui,/data-fusion-cell/);
  assert.match(ui,/Go to cell/);
  assert.match(ui,/onpaste/);
  assert.match(ui,/this\.saveCells/);
  assert.match(ui,/exportHtml\(\):string/);
  assert.match(main,/fusion\.exportHtml\(\)/);
});
test('MCP binary still imports shared embedded types',()=>{
  assert.match(bin,/mod fusion;/);
});
