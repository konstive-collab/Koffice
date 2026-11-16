import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const model=readFileSync('src-tauri/src/fusion.rs','utf8');
const links=readFileSync('src-tauri/src/fusion_links.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const main=readFileSync('src/main.ts','utf8');
const ui=readFileSync('src/fusion-ui.ts','utf8');

test('Link metadata is optional for old Writer embedded sheets',()=>{
  assert.match(model,/pub struct SheetLink/);
  assert.match(model,/skip_serializing_if = "Option::is_none"/);
  assert.match(model,/link: None/);
  assert.match(links,/serialization_preserves_link_metadata_and_legacy_embeds/);
});
test('Links detect changes and reject overwriting local edits',()=>{
  assert.match(links,/pub fn status/);
  assert.match(links,/pub fn refresh/);
  assert.match(links,/clean_embedded_revision/);
  assert.match(links,/Local embedded edits detected/);
  assert.match(links,/local_edits_block_refresh_and_allow_detach/);
});
test('New or opened workbooks invalidate session-bound links',()=>{
  assert.match(lib,/sheets_generation: AtomicU64/);
  assert.match(lib,/sheets_generation\.fetch_add/);
  assert.match(links,/source_token/);
  assert.match(links,/old_session_never_silently_relinks_and_explicit_reconnect_works/);
});
test('Linked APIs are registered in Rust Tauri',()=>{
  for(const name of ['writer_embed_linked_sheet','writer_link_status','writer_refresh_link','writer_reconnect_link','writer_detach_link']){
    assert.ok(lib.includes('fn '+name+'('),name);
    assert.ok(lib.includes(name),name);
  }
});
test('UI offers explicit sync, inspect, detach, reconnect',()=>{
  assert.match(main,/Link active Sheets/);
  for(const name of ['writer_embed_linked_sheet','writer_link_status','writer_refresh_link','writer_reconnect_link','writer_detach_link'])assert.ok(main.includes(name));
  for(const selector of ['data-fusion-check','data-fusion-refresh','data-fusion-reconnect','data-fusion-detach'])assert.ok(ui.includes(selector));
});
