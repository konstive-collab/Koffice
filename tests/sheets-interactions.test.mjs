import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
const code=readFileSync('src/sheets-interactions.ts','utf8');
const workbook=readFileSync('src/sheets-workbench.ts','utf8');
const frontend=readFileSync('src/main.ts','utf8');
const native=readFileSync('src-tauri/src/sheet_tools.rs','utf8');
const bridge=readFileSync('src-tauri/src/lib.rs','utf8');
const compile=content=>ts.transpileModule(content,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText;
const moduleUrl=code=>'data:text/javascript;base64,'+Buffer.from(code,'utf8').toString('base64');
const wbUrl=moduleUrl(compile(workbook));
const compiled=compile(code).replaceAll("'./sheets-workbench'","'"+wbUrl+"'").replaceAll('"./sheets-workbench"','"'+wbUrl+'"');
const funcs=await import(moduleUrl(compiled));
test('filter finds sparse matching rows without mutating workbook cells',()=>{
 const cells={A1:'Name',B1:'Grade',B2:'Excellent',B14:'excellent',B10000:'EXCELLENT',A3:'Other'};
 const snapshot=structuredClone(cells);
 assert.deepEqual(funcs.matchingRows(cells,{column:'b',query:'exce'}),[2,14,10000]);
 assert.deepEqual(funcs.matchingRows(cells,{column:'B',query:'missing'}),[]);
 assert.deepEqual(cells,snapshot);
});
test('selection works in reverse direction and outside initial viewport',()=>{
 const selection={sheet:0,anchor:'C100',focus:'A98'};
 assert.equal(funcs.selectedRect(selection).range,'A98:C100');
 assert.equal(funcs.selectedCellCount(selection),9);
 assert.equal(funcs.isSelected(selection,0,'B99'),true);
 assert.equal(funcs.isSelected(selection,1,'B99'),false);
 assert.equal(funcs.isSelected(selection,0,'D99'),false);
});
test('TSV export includes blanks and limits large selections',()=>{
 const selection={sheet:0,anchor:'A1',focus:'B2'};
 const cells={A1:'Apple',B1:'1',A2:'Pear'};
 assert.equal(funcs.rangeAsTsv(cells,selection),'Apple\t1\nPear\t');
 assert.throws(()=>funcs.rangeAsTsv({}, {sheet:0,anchor:'A1',focus:'ZZ10000'}),/5,000/);
});
test('native replace is revision-guarded, bounded, atomic and excludes formulas',()=>{
 assert.match(native,/pub fn replace_text/);
 assert.match(native,/value\.starts_with\('='\)/);
 assert.match(native,/book\.revision!=expected/);
 assert.match(native,/replace_text\(&mut b,3,0/);
 assert.match(native,/replacement_respects_bounds_and_preserves_formulas/);
 assert.match(native,/replacement_is_atomic_on_bad_limits_and_revision/);
 assert.match(bridge,/fn workbook_replace_text/);
 assert.match(frontend,/invoke<\[Book,number\]>\('workbook_replace_text'/);
});
test('filter UI and selection are connected to the visible viewport',()=>{
 for(const term of ['id="sheet-filter"','id="sheet-filter-clear"','id="sheet-copy"','id="sheet-selection-label"','id="sheet-replace"','displayedRows.map','selected-cell'])assert.ok(frontend.includes(term),term);
 assert.match(frontend,/input\.onclick=event=>/);
 assert.match(frontend,/event\.shiftKey/);
 assert.match(frontend,/rangeAsTsv\(b\.cells,sheetSelection\)/);
});
