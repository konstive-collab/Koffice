import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const base=readFileSync(new URL('../src-tauri/src/base.rs',import.meta.url),'utf8');
const lib=readFileSync(new URL('../src-tauri/src/lib.rs',import.meta.url),'utf8');
const ui=readFileSync(new URL('../src/base-ui.ts',import.meta.url),'utf8');
const cfg=JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json',import.meta.url),'utf8'));
test('database diagnostics, backup and report commands wired through Rust and Tauri',()=>{
  for(const name of ['base_integrity_check','base_backup','base_export_table_csv','base_group_report']){
    assert.ok(lib.includes(`fn ${name}(`),name);
    assert.ok(ui.includes(`'${name}'`),name);
  }
  assert.match(base,/fn backup_reports_and_full_export\(/);
  assert.match(base,/VACUUM INTO \?1/);
  assert.match(base,/if dst\.exists\(\)/);
  assert.match(base,/fn group_report\(/);
  assert.match(base,/fn export_table_csv\(/);
});
test('MSI version compatible with numeric-only Windows packaging',()=>{
  assert.match(cfg.version,/^\d+\.\d+\.\d+$/);
  assert.ok(cfg.bundle.targets.includes('msi'));
});