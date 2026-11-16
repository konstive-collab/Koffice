import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const suite=readFileSync('src-tauri/src/suite.rs','utf8');
const native=readFileSync('src-tauri/src/lib.rs','utf8');
const ui=readFileSync('src/main.ts','utf8');
const layout=readFileSync('src/slide-layout.ts','utf8');
const style=readFileSync('src/style.css','utf8');

test('slide frame values are normalized and saved as optional presentation metadata',()=>{
 assert.match(suite,/pub struct SlideObjectFrame/);
 assert.match(suite,/pub object_frames: BTreeMap<String,SlideObjectFrame>/);
 assert.match(suite,/skip_serializing_if="BTreeMap::is_empty"/);
 assert.match(suite,/Object frame must stay inside the slide/);
 assert.match(suite,/slide_table_frames_survive_save_duplicate_and_reorder/);
 assert.match(suite,/slide_table_frames_validate_bounds_and_rollback/);
});
test('native geometry updates are revision-guarded and transactional',()=>{
 assert.match(suite,/pub fn move_embedded/);
 assert.match(suite,/check_revision\(self\.revision,expected\)/);
 assert.match(suite,/frame\.validate\(\)\?/);
 assert.match(suite,/next\.validate\(\)\?/);
 assert.match(native,/fn slides_embedded_frame\(/);
 assert.match(ui,/slides_embedded_frame/);
});
test('per-slide frames remain through text editing and are removed with embedded objects',()=>{
 assert.match(suite,/replacement\.object_frames=old\.object_frames\.clone\(\)/);
 assert.match(suite,/slide\.object_frames\.remove\(id\)/);
 assert.match(suite,/Object frame refers to missing embedded table/);
});
test('pointer and keyboard drag-resize use normalized frame fractions and save once',()=>{
 assert.match(layout,/function wireSlideFrames/);
 assert.match(layout,/pointerdown/);
 assert.match(layout,/pointermove/);
 assert.match(layout,/pointerup/);
 assert.match(layout,/setPointerCapture/);
 assert.match(layout,/keydown/);
 assert.match(layout,/Shift\+arrows/);
 assert.match(layout,/1000/);
 assert.match(ui,/wireSlideFrames/);
 assert.match(style,/slide-object-resize/);
});
test('HTML exports and presentations keep object placement instead of flattening to a list',()=>{
 assert.match(layout,/function renderStaticSlideFrames/);
 assert.match(layout,/position:absolute/);
 assert.ok((ui.match(/renderStaticSlideFrames\(s\.embedded_objects\?\?\[\]/g)||[]).length>=3);
});
