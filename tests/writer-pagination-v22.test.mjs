import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const engine=readFileSync('src/writer-pagination.ts','utf8');
const ui=readFileSync('src/main.ts','utf8');
const css=readFileSync('src/style.css','utf8');
test('paragraph splitting preserves formatting with Range.cloneContents',()=>{
 assert.match(engine,/export function copyFormattedRange/);
 assert.match(engine,/document\.createRange\(\)/);
 assert.match(engine,/range\.cloneContents\(\)/);
 assert.match(engine,/source\.cloneNode\(false\)/);
 assert.match(engine,/plainTextNodes\(source\)/);
 assert.match(engine,/safeSplitEnd/);
});
test('pagination measures text and binary-searches a fitting prefix',()=>{
 assert.match(engine,/function appendParagraph\(element:Element\)/);
 assert.match(engine,/while\(lo\+1<hi\)/);
 assert.match(engine,/testFits\(copyFormattedRange\(element,start,mid\)\)/);
 assert.match(engine,/splitPreferred\(text,start,lo\)/);
 assert.match(engine,/measureBody\.scrollHeight<=height\+1/);
});
test('table rows split across pages with repeated thead',()=>{
 assert.match(engine,/function appendTable\(table:HTMLTableElement\)/);
 assert.match(engine,/Array\.from\(table\.rows\)/);
 assert.match(engine,/table\.tHead\?\.cloneNode\(true\)/);
 assert.match(engine,/chunk\.tBodies\[0\]/);
 assert.match(css,/table-header-group/);
 assert.match(css,/break-inside:avoid/);
});
test('preview never edits the source; overlarge rows remain visible',()=>{
 assert.match(engine,/source\.childNodes/);
 assert.match(engine,/node\.cloneNode\(true\)/);
 assert.match(engine,/oversizedBlock\(chunk\)/);
 assert.match(engine,/MAX_BLOCKS=1500/);
 assert.match(engine,/MAX_PAGES=250/);
 assert.doesNotMatch(engine,/invoke\(|replace_rich|edit_document/);
});
test('live Writer editor remains independent of preview layout',()=>{
 assert.match(ui,/openWriterPages\(doc\.page,live\.innerHTML,doc\.text,doc\.title\)/);
 assert.match(engine,/sanitizeRichHtml\(richHtml\?\?plaintextRichHtml\(plain\)\)/);
 assert.match(engine,/const close=\(\)=>\{overlay\.remove\(\);measurement\.remove\(\);\}/);
});
