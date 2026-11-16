import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const engine=readFileSync('src-tauri/src/engine.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/main.ts','utf8');
const page=readFileSync('src/writer-page.ts','utf8');
const rich=readFileSync('src/writer-rich.ts','utf8');
const css=readFileSync('src/style.css','utf8');
test('Writer page options are optional and legacy documents remain readable',()=>{
 assert.match(engine,/pub page: PageSettings/);
 assert.match(engine,/pub struct PageSettings/);
 assert.match(engine,/impl Default for PageSettings/);
 assert.match(engine,/page_settings_roundtrip_legacy_and_validation/);
});
test('Page options are validated and revision guarded before commit',()=>{
 assert.match(engine,/pub fn set_page/);
 assert.match(engine,/self.check_revision\(expected_revision\)/);
 assert.match(engine,/Page margins must be 5/);
 assert.match(engine,/page_setup_rejects_unsafe_geometry_atomically/);
 assert.match(lib,/fn writer_set_page/);
 assert.match(ui,/writer_set_page/);
});
test('Physical paper and margins are applied consistently',()=>{
 assert.match(page,/paperDimensions/);
 for(const paperName of ['a4','letter','legal'])assert.ok(page.includes("'"+paperName+"'"));
 assert.match(page,/cssPageStyle/);
 assert.match(page,/pageCss/);
 assert.match(ui,/writer-paper/);
 assert.match(css,/--writer-paper-width/);
 assert.match(ui,/cssPageStyle\(doc.page\?\?DEFAULT_PAGE\)/);
});
test('Page breaks and headers survive rich formatting and export',()=>{
 assert.match(rich,/case 'pagebreak'/);
 assert.match(rich,/section/);
 assert.match(ui,/data-format="pagebreak"/);
 assert.match(ui,/pagePrintableHtml/);
 assert.match(page,/escapePageText/);
 assert.match(css,/break-before:page/);
});
test('Page setup has editable paper sizes, orientation, margins and header-footer',()=>{
 for(const field of ['paper','landscape','margin_top_mm','margin_right_mm','margin_bottom_mm','margin_left_mm','header','footer'])
  assert.ok(page.includes("name=\""+field+"\""),field);
 assert.match(page,/onApply\(next\)/);
 assert.match(ui,/openPageSetup/);
});
