import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const engine=readFileSync('src-tauri/src/engine.rs','utf8');
const ui=readFileSync('src/main.ts','utf8');
const rich=readFileSync('src/writer-rich.ts','utf8');
const style=readFileSync('src/style.css','utf8');

test('native Writer roundtrips formatting and opens old plain documents',()=>{
 assert.match(engine,/pub rich_html: Option<String>/);
 assert.match(engine,/ReplaceRich \{ text: String, html: String \}/);
 assert.match(engine,/formatted_text_roundtrip_and_legacy_compatibility/);
 assert.match(engine,/candidate\.rich_html=None/);
});
test('unsafe HTML is stripped before any rendering or export',()=>{
 assert.match(rich,/sanitizeRichHtml/);
 assert.match(rich,/DOMParser/);
 assert.match(rich,/const wrapper=ALL\.has\(tag\)\?doc\.createElement\(tag\):doc\.createDocumentFragment/);
 assert.match(rich,/if\(\['script','style','iframe'/);
 assert.match(engine,/markup_validation_rejects_active_content_without_mutation/);
 assert.match(ui,/sanitizeRichHtml\(doc\.rich_html\?\?plaintextRichHtml\(doc\.text\)\)/);
});
test('Writer has editable rich document and functional formatting actions',()=>{
 assert.match(ui,/contenteditable="true"/);
 assert.match(ui,/class="writer-rich-editor"/);
 assert.match(ui,/replace_rich/);
 assert.match(rich,/formatSelection/);
 for(const name of ['bold','italic','underline','strike','h1','h2','paragraph','quote','bullet','number','table','undo','redo'])assert.ok(rich.includes("'"+name+"'"),name);
 assert.match(style,/\.writer-rich-editor table/);
});
test('Print and HTML export use rich content and Fusion remains enabled',()=>{
 assert.match(ui,/<article>\$\{sanitizeRichHtml\(doc\.rich_html/);
 assert.match(ui,/class="writer-print-body"/);
 assert.match(ui,/\$\{fusion\.exportHtml\(\)\}/);
});
