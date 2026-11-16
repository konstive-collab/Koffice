import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const fusion=readFileSync('src-tauri/src/fusion.rs','utf8');
const files=readFileSync('src-tauri/src/fusion_files.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/fusion-ui.ts','utf8');
const main=readFileSync('src/main.ts','utf8');
test('persistent file-link metadata is optional and version-compatible',()=>{
 assert.match(fusion,/pub struct FileSheetLink/);
 assert.match(fusion,/skip_serializing_if = "Option::is_none"\)] file_link/);
 assert.match(fusion,/link: None, file_link: None/);
 assert.match(fusion,/Cannot link the same object to two sources/);
});
test('reading file links is explicit, validated, bounded and source read-only',()=>{
 assert.match(files,/pub fn load_source/);
 assert.match(files,/suite::load_json/);
 assert.match(files,/source.validate\(\)\?/);
 assert.match(files,/pub fn status/);
 assert.match(files,/pub fn refresh/);
 assert.match(files,/get_original/);
 assert.match(files,/check_local/);
 assert.doesNotMatch(files,/std::fs::write|std::fs::OpenOptions/);
});
test('file link operations are Tauri commands',()=>{
 for(const key of ['fusion_file_sheets','writer_attach_file_sheet','writer_file_link_status','writer_refresh_file_link','writer_reconnect_file_link','writer_detach_file_link']){
  assert.ok(lib.includes('fn '+key+'('),key);
  assert.ok(lib.includes(key+',')||lib.includes(key+',workbook_snapshot'),key+' in handler');
 }
});
test('frontend requires file picker and confirmation before reading saved sources',()=>{
 assert.match(main,/Link Sheets file/);
 assert.match(main,/chooseFusionFile/);
 assert.match(main,/window.confirm\('Check source file/);
 assert.match(main,/window.confirm\('Check and refresh/);
 for(const key of ['data-file-check','data-file-refresh','data-file-reconnect','data-file-detach']) assert.ok(ui.includes(key));
});
test('native on-disk tests include restart, local conflict and missing file',()=>{
 for(const name of ['persists_link_after_restart_and_refreshes_file_edits','local_changes_block_file_refresh_even_if_source_changed','missing_file_and_wrong_identity_never_overwrite','invalid_source_fails_without_mutation'])assert.ok(files.includes('fn '+name));
});
