import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
const read = p => readFileSync(new URL(`../${p}`, import.meta.url), 'utf8');
test('app branding and Tauri scripts', () => {
  const p = JSON.parse(read('package.json'));
  const config = JSON.parse(read('src-tauri/tauri.conf.json'));
  assert.equal(config.productName, 'Koffice');
  assert.ok(p.scripts.build && p.scripts.tauri);
});
test('Rust API commands are exposed and implemented', () => {
  const backend = read('src-tauri/src/lib.rs');
  const frontend = read('src/main.ts');
  for (const cmd of ['snapshot','new_document','open_document','save_document','edit_document','rename_document']) {
    assert.match(backend, new RegExp(`fn ${cmd}\\(`));
    assert.ok(frontend.includes(`'${cmd}'`));
  }
});
test('all required application source files exist', () => {
  for(const path of ['index.html','src/style.css','src-tauri/src/engine.rs','src-tauri/capabilities/default.json'])
    assert.ok(existsSync(new URL(`../${path}`, import.meta.url)), path);
});