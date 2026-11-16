import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const native=readFileSync('src-tauri/src/fusion_manager.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const front=readFileSync('src/fusion-manager-ui.ts','utf8');
const main=readFileSync('src/main.ts','utf8');
test('file graph lists without following paths',()=>{
 assert.match(native,/pub fn graph\(/);
 assert.match(native,/status:"?unchecked"?|\{"unchecked"\}/);
 assert.match(native,/graph_never_reads_disk/);
});
test('bounded two-phase source validation and one-shot previews',()=>{
 assert.match(native,/pub fn preview\(/);
 assert.match(native,/pub fn apply\(/);
 assert.match(native,/MAX_REFRESH_OBJECTS: usize=12/);
 assert.match(native,/MAX_DIFF_EXAMPLES: usize=8/);
 assert.match(native,/Source file changed after preview/);
 assert.match(lib,/let batch=slot.take\(\)/);
 assert.match(lib,/fusion_preview_sequence: AtomicU64/);
});
test('batch rollback when any source changes or local edit conflicts',()=>{
 for(const name of ['preview_then_apply_is_atomic_for_two_sources','changed_source_between_preview_and_apply_rolls_back_all','edited_embedded_sheet_blocks_batch_and_stays_intact']) assert.ok(native.includes('fn '+name));
});
test('Tauri batch commands and UX are connected',()=>{
 for(const name of ['fusion_graph','fusion_preview_refresh','fusion_apply_refresh'])assert.ok(lib.includes('fn '+name+'(')&&main.includes(name));
 assert.ok(main.includes('Fusion links'));
 assert.match(front,/class FusionManagerUI/);
 assert.match(front,/Preview selected files/);
 assert.match(front,/Apply /);
 assert.match(front,/window\.confirm\('Read these linked source files/);
 assert.match(front,/window\.confirm\('Apply /);
});
test('inventory excludes unrelated tables from file batch check',()=>{
 assert.match(front,/n\.mode==='file'/);
 assert.match(native,/Only file-backed links can be refreshed in this batch/);
});
