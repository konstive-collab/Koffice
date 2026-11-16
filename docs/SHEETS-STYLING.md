# Koffice 0.25 — persisted cell formatting and frozen panes

**Kodi Sheets** now stores a minimal but genuinely persisted presentation model in native workbook files.

Select a range with click and Shift+click, then choose **Format cells**. You can set bold, italic, left/center/right alignment and a preset background (none/yellow/green/blue/red/gray) for up to 5,000 cells in one atomic update. Styles remain associated with cell addresses and are saved in `.ksheet`, including reopening and embedding a workbook in Fusion objects.

Choose **Freeze panes** to pin the first 0–3 rows and 0–3 columns. The grid window displays these pinned rows/columns alongside the scrolling viewport; this is a persistent document setting, not a session-only toggle.

The Rust core validates all style tokens and addresses, rejects oversized batches, checks revisions and commits atomically. Old workbook files without `styles` and `freeze` continue to load with defaults.

**Important limitations:** the palette and formatting set are deliberately small. There are no arbitrary fonts, borders, number/date formats, conditional formatting or XLSX interoperability yet. Frozen panes are implemented as pinned cells in the bounded 40×16 rendered viewport rather than pixel-perfect spreadsheet scrolling. Sorting still moves values without rebasing styles or formulas: formatting remains tied to physical addresses, not to records. Back up a workbook before structural data operations.
