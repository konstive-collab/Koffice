import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const desktop=readFileSync('src-tauri/src/desktop.rs','utf8');
const lib=readFileSync('src-tauri/src/lib.rs','utf8');
const main=readFileSync('src/main.ts','utf8');
const css=readFileSync('src/style.css','utf8');
const conf=JSON.parse(readFileSync('src-tauri/tauri.conf.json','utf8'));

test('native launch supports only the four known products',()=>{
 for(const product of ['writer','sheets','slides','base']){
  assert.ok(desktop.includes('"'+product+'"'));
 }
 assert.match(desktop,/pub fn desktop_launch/);
 assert.match(desktop,/std::env::current_exe/);
 assert.match(desktop,/std::process::Command::new\(executable\)/);
 assert.match(desktop,/Unknown Koffice application/);
 assert.match(desktop,/cli_modes_are_strict/);
 const production=desktop.split('#[cfg(test)]')[0];
 assert.doesNotMatch(production,/cmd\.exe|powershell|\/bin\/sh|\.arg\("-c"\)/);
});
test('Tauri registers launcher commands and sets app title',()=>{
 assert.match(lib,/mod desktop;/);
 assert.match(lib,/desktop::desktop_mode,desktop::desktop_launch/);
 assert.match(lib,/set_title\(mode\.window_title\(\)\)/);
});
test('UI routes launched modes to independent workspaces',()=>{
 assert.match(main,/invoke<DesktopMode>\('desktop_mode'\)/);
 assert.match(main,/applyDesktopMode\(mode\)/);
 assert.match(main,/active=mode/);
 assert.match(main,/if\(mode==='suite'\)showAppLauncher\(\)/);
 assert.match(main,/invoke<string>\('desktop_launch',\{kind\}\)/);
 assert.match(main,/className='desktop-launcher'/);
});
test('single-product mode hides integrated navigation, Suite keeps it',()=>{
 assert.match(main,/if\(desktopMode!=='suite'\)return/);
 assert.match(main,/desktopMode==='suite'&&!e\.shiftKey/);
 assert.match(css,/body\[data-app-mode\]:not\(\[data-app-mode="suite"\]\) \.sidebar\{display:none\}/);
 assert.match(css,/\.desktop-app-grid/);
 assert.match(css,/prefers-reduced-motion/);
});
test('version and packaged app remain consistent',()=>{
 assert.equal(conf.version,JSON.parse(readFileSync('package.json','utf8')).version.split('-')[0]);
 assert.equal(conf.bundle.active,true);
 assert.ok(readFileSync('src-tauri/Cargo.toml','utf8').includes('version = "'+JSON.parse(readFileSync('package.json','utf8')).version+'"'));
});
