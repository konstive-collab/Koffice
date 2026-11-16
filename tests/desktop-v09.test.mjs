import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const main=readFileSync('src/main.ts','utf8');
const css=readFileSync('src/style.css','utf8');
const cargo=readFileSync('src-tauri/Cargo.toml','utf8');
const ci=readFileSync('.github/workflows/ci.yml','utf8');

test('Tauri desktop defaults to koffice, not MCP',()=>{
  assert.match(cargo,/default-run\s*=\s*"koffice"/);
});
test('workspace shortcuts are registered',()=>{
  for(const key of ["key==='n'","key==='o'","key==='s'","'1','2','3','4'"])
    assert.ok(main.includes(key), 'Missing shortcut: '+key);
});
test('preference bounds and storage fallback are present',()=>{
  assert.match(main,/Math\.max\(\.8,Math\.min\(1\.35/);
  assert.ok(main.includes('catch{return {compact:false,scale:1};}'));
  assert.match(main,/localStorage\.setItem/);
});
test('responsive and keyboard accessibility contracts',()=>{
  assert.match(css,/@media\(max-width:650px\)/);
  assert.match(css,/:focus-visible/);
  assert.match(css,/prefers-reduced-motion/);
});
test('all three desktop platforms have build jobs',()=>{
  assert.match(ci,/windows-desktop:/);
  assert.match(ci,/linux-desktop:/);
  assert.match(ci,/macos-desktop:/);
  assert.match(ci,/--bundles deb/);
  assert.match(ci,/--bundles app/);
});
