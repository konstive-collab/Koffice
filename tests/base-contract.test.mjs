import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const base=readFileSync(new URL('../src-tauri/src/base.rs',import.meta.url),'utf8');
const lib=readFileSync(new URL('../src-tauri/src/lib.rs',import.meta.url),'utf8');
const ui=readFileSync(new URL('../src/base-ui.ts',import.meta.url),'utf8');
const main=readFileSync(new URL('../src/main.ts',import.meta.url),'utf8');
test('Base is wired to all the CRUD and query commands',()=>{
 for(const name of ['base_create','base_open','base_tables','base_create_table','base_insert','base_update','base_delete','base_browse_page','base_select']){
  assert.ok(lib.includes(`fn ${name}(`),name);assert.ok(ui.includes(`'${name}'`),name);
 }
 assert.ok(main.includes('nav-base'));assert.ok(main.includes('base.render()'));
});
test('parameterized SQL, query limits, safety tests',()=>{
 assert.match(base,/params_from_iter/);assert.match(base,/fn rejects_sql_injection_and_bad_fields/);
 assert.match(base,/fn crud_and_reopen/);assert.match(base,/fn pagination_cap/);
 assert.match(base,/!stmt.readonly\(\)/);assert.match(base,/const MAX_ROWS: usize = 500/);
});