import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const suite=readFileSync('src-tauri/src/suite.rs','utf8');
const native=readFileSync('src-tauri/src/base_slide_links.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const main=readFileSync('src/main.ts','utf8');
const base=readFileSync('src/base-ui.ts','utf8');
test('slide source metadata is optional for legacy presentations',()=>{
  assert.match(suite,/pub struct BaseSlideSource/);
  assert.match(suite,/pub base_source: Option<BaseSlideSource>/);
  assert.match(suite,/skip_serializing_if="Option::is_none"/);
  assert.match(native,/legacy_slide_json_remains_loadable/);
});
test('slide source is attached to the slide, not to its array index',()=>{
  assert.match(suite,/replacement.base_source=old.base_source.clone\(\)/);
  assert.match(native,/move_duplicate_and_delete_keep_link_on_its_slide/);
});
test('native link uses bounded Base snapshot, read-on-confirm and rejects conflicts',()=>{
  assert.match(native,/base_snapshot_slide/);
  assert.match(native,/fn source_path/);
  assert.match(native,/slide.content\(\)!=original.clean/);
  assert.match(native,/local_edits_block_refresh_and_keep_source/);
  assert.match(native,/base_link_roundtrip_after_restart_and_manual_refresh/);
  assert.match(native,/stale_revision_and_missing_source_do_not_mutate_presentation/);
});
test('commands are registered and exposed through UI',()=>{
  for(const name of ['base_link_to_slides','slides_base_link_status','slides_base_link_refresh','slides_base_link_reconnect','slides_base_link_detach']){
    assert.ok(lib.includes('fn '+name+'('),name);
    assert.ok(main.includes(name),name);
  }
  assert.match(base,/id="base-link-slides"/);
  assert.match(main,/Check Base/);
  assert.match(main,/Refresh Base/);
  assert.match(main,/Relink Base/);
  assert.match(main,/Detach Base/);
});
test('UI explicitly confirms stored path before reading database',()=>{
  assert.match(main,/window.confirm\('Read linked Base source/);
  assert.match(main,/window.confirm\('Refresh this slide from the linked database/);
  assert.match(main,/database_path/);
  assert.match(native,/moved_database_can_be_relinked_after_explicit_selection/);
});
