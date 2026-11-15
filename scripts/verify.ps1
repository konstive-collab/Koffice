$ErrorActionPreference = 'Stop'
npm install
if ($LASTEXITCODE -ne 0) { throw 'npm install failed' }
npm test
if ($LASTEXITCODE -ne 0) { throw 'Node tests failed' }
npm run build
if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed' }
cargo test --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
Write-Host 'Koffice validation passed.'