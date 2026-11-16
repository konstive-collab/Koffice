import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const ui=readFileSync('src/main.ts','utf8');
const view=readFileSync('src/writer-pagination.ts','utf8');
const css=readFileSync('src/style.css','utf8');
const rich=readFileSync('src/writer-rich.ts','utf8');
test('Paginated preview is explicitly opened from Writer without modifying the editor',()=>{
 assert.match(ui,/id="writer-pages"/);
 assert.match(ui,/openWriterPages\(doc\.page,live\.innerHTML,doc\.text,doc\.title\)/);
 assert.match(view,/export function openWriterPages/);
 assert.doesNotMatch(view,/edit_document|replace_rich|invoke\(/);
});
test('Blocks are measured on an off-screen full-size paper, including footer and header',()=>{
 assert.match(view,/writer-page-measure/);
 assert.match(view,/measurement\.append\(measureHeader,measureBody,measureFooter\)/);
 assert.match(view,/measureBody\.clientHeight/);
 assert.match(view,/measureBody\.scrollHeight/);
 assert.match(view,/getBoundingClientRect\(\)/);
 assert.match(view,/makePage\(settings,built\.length\+1,title\)/);
});
test('Explicit break and oversized nodes are handled without content loss',()=>{
 assert.match(view,/tagName\.toLowerCase\(\)==='section'/);
 assert.match(view,/writer-page-overflow/);
 assert.match(view,/writer-preview-cutoff/);
 assert.match(view,/MAX_BLOCKS=1500/);
 assert.match(view,/MAX_PAGES=250/);
 assert.match(rich,/case 'pagebreak'/);
});
test('Navigation, zoom, accessibility and keyboard support are present',()=>{
 for(const term of ['Previous','Next','Edit document','aria-live','aria-label','PageDown','PageUp','Escape','writer-pages-zoom'])assert.ok(view.includes(term),term);
 assert.match(css,/\.writer-preview-paper/);
 assert.match(css,/\.writer-pages-scroll/);
 assert.match(css,/prefers-reduced-motion|:focus-visible/);
});
test('Legacy document and native page settings remain separate from read-only view',()=>{
 assert.match(view,/sanitizeRichHtml\(richHtml\?\?plaintextRichHtml\(plain\)\)/);
 assert.match(view,/page\?\?DEFAULT_PAGE/);
 assert.match(view,/const close=\(\)=>\{overlay\.remove\(\);measurement\.remove\(\);\}/);
});
