# Koffice v0.9 — Desktop experience and cross-platform delivery

The focus of Koffice is **performance, usability, cross-platform desktop operation and reliable documents**. AI automation remains optional.

## Changes
- Fix Windows Tauri packaging after MCP introduced a second Rust binary: Cargo `default-run = "koffice"`.
- Compact navigation sidebar; toggle with the toolbar button or Ctrl/Cmd+Shift+B.
- Workspace zoom 80–135%, with toolbar controls. Reset to 100% with Ctrl/Cmd+Shift+0.
- Keep sidebar and zoom preferences locally, without requiring an account.
- Native file keyboard shortcuts: Ctrl/Cmd+N new, Ctrl/Cmd+O open, Ctrl/Cmd+S save, Ctrl/Cmd+Shift+S save as.
- Ctrl/Cmd+1/2/3/4 to switch Writer/Sheets/Slides/Base.
- Improved narrow-window layout, visible keyboard focus and reduced-motion accessibility.
- CI builds Windows NSIS/MSI, Linux DEB and a macOS app bundle, with separate Rust/frontend tests. **Actual packaging success is determined by the GitHub Actions run.**

## Roadmap priorities
1. Robust DOCX/XLSX/PPTX interoperability with loss detection and round-trip tests.
2. Editor performance benchmarks: 10k+ cells, large documents, slide navigation, startup and memory.
3. Desktop UI: keyboard-first workflow, resizable panes, better selection, viewport virtualization.
4. Cross-platform releases: installer signing/notarization, update channel and automated smoke tests.
5. Databases: visual forms, relational relationships, accessible reports.
6. Optional AI and MCP integration.

**Known limitations:** Koffice remains an experimental alpha; Writer uses plain text, Sheets has a limited formula engine, and Slides lacks rich slide objects. Cross-platform CI packaging does not imply user-level UI testing or broad compatibility. Back up important data. macOS bundles are unsigned; Windows installers are unsigned.