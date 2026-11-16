# Koffice 0.18 — Editor Separation, Phase 1

**Purpose:** the first concrete step away from one always-integrated prototype UI toward four standalone Kodi Office products.

## What works

The installed Koffice executable now accepts these arguments:

| Command | Product |
| --- | --- |
| `koffice` | Kodi Office Suite and application launcher |
| `koffice --app writer` | Kodi Writer |
| `koffice --app sheets` | Kodi Sheets |
| `koffice --app slides` | Kodi Slides |
| `koffice --app base` | Kodi Base |

The default Suite window shows an application launcher on startup. Choose any product to start a **second native process** of the installed executable, with an application-specific window title, workspace selection and product accent. Each new process owns its own Rust `Workspace`, document revisions, unsaved edits and Base database connection; **these are not just tabs in a shared memory window**.

In a standalone product, the application-navigation sidebar is hidden and Ctrl+1..4 no longer silently switches editors. The **Applications** toolbar button launches the other independent editors. A Suite window retains the existing navigation tabs, cross-editor Fusion tools and backward-compatible workflows.

No shell or user-provided executable path is involved. The native launcher uses `std::env::current_exe()` and permits only four exact app identifiers. Neither file operations nor arbitrary commands are accepted through the launch action.

## Developer workflow

On Windows from the repository root, after building Rust, run e.g. `src-tauri\target\debug\koffice.exe --app writer` or `src-tauri\target\release\koffice.exe --app sheets`. Installed builds use the normal installed executable path with the same `--app` arguments.

Linux: `koffice --app slides` (if the executable is on PATH). macOS: run the executable from the installed `.app/Contents/MacOS/` directory with `--app base` as its argument. Regular icon launching opens the Suite launcher.

## Known limitations

This is **Phase 1**, not four independently packaged products. There is one executable, one installation, one shared compiled codebase, and four process modes. Separate application shortcuts, installers, file associations and app-specific process entry points are later stages.

Fusion is currently fully functional inside the integrated Suite process. Standalone processes **do not share unsaved in-memory sheets, slides or documents**; they exchange saved files. We do not claim that an active Sheets window automatically synchronizes a separate Writer window. A future IPC/document service is needed before seamless live cross-process embedding can be advertised.

Writer is still a plain-text-centric alpha prototype; Sheets, Slides and Base likewise lack many capabilities expected from full office suites. The next substantial milestones are rich-text pages, text styles/formatting, scalable spreadsheet UI, formula engine expansion, menus/ribbons, Undo/Redo, and import/export of established office formats.

## Verification

Rust unit tests cover exact CLI mode selection and rejection of arbitrary launch identifiers. Node contract tests cover the native launcher, startup routing, standalone navigation restrictions, Suite compatibility, app-specific shells and build targets. CI exercises frontend tests and builds Windows, Linux and macOS installers. A successful CI compile does not replace hands-on visual QA.
