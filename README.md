## v0.6 alpha

Database diagnostics (`PRAGMA integrity_check`), safe SQLite `VACUUM INTO` backups, full-table CSV export (up to 10,000 rows) and grouped read-only COUNT/SUM/AVG reports are available through Base UI and typed Tauri commands. The Windows installer version is numeric to satisfy MSI packaging rules. Other Office-native format support is not yet implemented.

# Koffice ☕ — open-source, AI-native office suite

**Koffice** is a local-first office suite built in **Rust + Tauri**. This community alpha ships three early editors sharing a Rust backend. The goal is robust document processing from both the desktop interface and AI agents using the same typed commands.

## Available in v0.2.0-alpha.1

| App | Working features | Native format |
|---|---|---|
| Writer | Plain-text editing; Markdown-like formatting shortcuts; title, document operations, Save/Open, TXT export, print/PDF via system print | `.kdoc` |
| Sheets | Editable 12×35 viewport; cells stored in a larger grid; multiple sheets; arithmetic, cell references and SUM formulas; CSV export (current sheet) | `.ksheet` |
| Slides | Create/edit/delete slides; speaker notes; presentation view; print/PDF via system print | `.kslide` |

**Important:** these are early editors, not complete substitutes for Word, Excel or PowerPoint. DOCX, XLSX, PPTX import/export, advanced formatting, images, charts, collaboration, MCP and Python SDK are not implemented yet. The native JSON formats are for Koffice, not Microsoft file types. Keep backups of important documents. Do not call this a stable release.

## Run on Windows

Install **Rust stable**, **Node.js 20+**, **Microsoft C++ Build Tools**, and **WebView2**. Then, from the project directory:

```powershell
npm install
npm run tauri dev
```

Run tests:

```powershell
npm test
cd src-tauri
cargo test
```

Build an installer (requires a configured Windows environment):

```powershell
npm run tauri build
```

## Architecture

- `src-tauri/src/engine.rs`: versioned Writer document operations and checked file replacement.
- `src-tauri/src/suite.rs`: typed Sheets/Slides domain operations, formula evaluation, validation and structured storage.
- `src-tauri/src/lib.rs`: Tauri application commands; the UI must not manipulate Rust internals directly.
- `src/`: TypeScript UI.
- `.github/workflows/ci.yml`: automated test/build checks.

AI access is a **design goal**, not an implemented network service. Future JSON-RPC/MCP adapters must call the same domain engines, with permission checks, auditable actions and safe transactions.

## Security and known limitations

- The alpha only opens Koffice JSON documents for its own native formats.
- Formula parser supports numeric arithmetic, single-cell references and `SUM(A1:A5)`; not the full spreadsheet function language.
- CSV export covers the visible sheet's first 26 columns and currently lacks complex quoting/formatting options.
- Replacing an existing file uses a same-folder temporary file and backup/rollback strategy. Power loss and filesystems still require recovery tests.
- Do not treat unsaved edits or renderer notifications as persistent commits until Save has completed.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). We welcome tests, UX changes, bug reports and proposals for standards-compatible document engines. Licensed under [MIT](LICENSE).

[Русская документация](README.ru.md)

## v0.3 alpha development update
- **Writer:** find and replace the selected match; basic text formatting tokens and TXT export.
- **Sheets:** atomic multi-cell paste (TSV from clipboard), CSV import into active sheet (up to 5 MiB / 20,000 edits), CSV export, rename/delete sheets, `SUM`, `AVERAGE`, `COUNT`, `MIN`, `MAX`, arithmetic expressions and direct cell references.
- **Slides:** duplicate slides and reorder slides with Up/Down controls, in addition to editing, notes and presentation preview.
- **Core:** revision-checked, validated batch operations, with changes rolled back on failure.

These are **alpha** functions and have limitations. CSV importer uses comma-delimited UTF-8; complex spreadsheet formulas and Microsoft Office file formats are not supported. The Writer is a plain-text editor (the formatting buttons insert markup characters, not WYSIWYG styles). The app has no MCP server yet.

Build/test status is published in GitHub Actions; structural contract tests are not a substitute for running the Rust and UI test suites.

## Koffice Base (v0.4 alpha)

SQLite-based `.kdb` databases with a native Rust API: create/open, typed schemas, multiple tables, record CRUD, text search, read-only SQL explorer, CSV export of visible results (500-row cap). Every successful insert/update/delete is saved automatically. The SQL explorer is read-only; user-provided values in CRUD operations are parameterized.

**Limitations:** no MS Access MDB/ACCDB import, no forms, reports, visual relationships or migrations yet. Keep backups. No external database connection support yet.


## Koffice Base v0.5: database power tools

- Transactional CSV import to an existing table using its exact column names (5 MB / 10,000 rows per import). Any error rolls back the whole import. CSV headers must not contain the auto-generated `id` column.
- Index management (single or multi-column, optional unique constraint), with index listing and deletion.
- Database paging in 500-row blocks and text-column search.
- **Base → Sheets** copies a 500-row database snapshot into a new unsaved workbook; exporting large datasets will require future streaming APIs.
- New Rust tests cover rollback, quoted CSV, index uniqueness, pagination, and existing CRUD paths.

These features are experimental and do not yet provide Access file compatibility or graphical relational forms. Save backup copies before using early alpha builds.

CI includes a Windows desktop build job. If successful, Windows installers are downloadable from the corresponding GitHub Actions run artifacts (not a signed release).