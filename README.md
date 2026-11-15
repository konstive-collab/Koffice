# Koffice ☕

**Koffice** is an open-source, local-first office suite built with **Rust + Tauri**. Its AI-native architecture exposes typed document operations for both the desktop UI and future AI agents.

## Apps
- **Writer:** plain-text document editor, native `.kdoc` file format and save/reopen.
- **Sheets:** native `.ksheet` workbooks, multiple sheets, grid editing, arithmetic, cell references and SUM formulas.
- **Slides:** native `.kslide` presentations, add/edit/delete slides, speaker notes and presenter view.

> **Alpha release:** These formats are not DOCX, XLSX or PPTX. Their import/export is planned, not currently supported. PDF export is currently implemented through the operating system print dialog. Do not use this alpha for critical documents without backups.

## Build
Install Rust, Node.js 20+, platform-specific Tauri 2 dependencies, then:

```sh
npm install
npm run tauri dev
npm test
cd src-tauri && cargo test
```

## Architecture
The `src-tauri/src` engine owns file validation, revision checks and document mutations. The frontend calls typed Tauri commands; future SDK / MCP integrations should use the same engine, not mutate files directly.

## Contributing
MIT license. Bugs, designs, tests, features and pull requests welcome. Read CONTRIBUTING.md.

### Русский
Koffice — открытый офисный пакет на Rust и Tauri. В ранней версии доступны Writer, Sheets и Slides с собственными форматами; совместимость с форматами Microsoft Office находится в планах.
