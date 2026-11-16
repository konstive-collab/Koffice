# Koffice v0.24 — Filter, select, copy and replace in Sheets

Kodi Sheets now offers four connected workflows in the native editor.

**Filter rows:** choose a column and a text fragment. A case-insensitive read-only filter shows only rows whose raw cell text contains the fragment, across the supported 10,000-row sheet. The original workbook, formulas, row addresses and save data remain unchanged. **Show all** restores the normal viewport. Filtering is per current sheet and is not yet persisted in `.ksheet`.

**Range selection:** click an input cell to select it, then Shift+click another cell to select a rectangle, even when the endpoints are in reverse order. The selected range is highlighted and shown below the grid.

**Copy range:** copy up to 5,000 selected cells to the clipboard as tab-separated values (TSV), with empty cells preserved. If system clipboard access is unavailable, Koffice shows text for manual copying. It copies formula expressions, not computed values.

**Replace text:** apply a case-sensitive literal replacement across at most 20,000 cells. Rust skips formula expressions, checks expected workbook revision and updates matching cells atomically. A zero-match operation does not change revision. Search/replacement strings are capped at 256 characters.

All destructive actions require confirmation. Sorting from 0.23 remains formula-protected. You can combine a temporary row filter with individual cell edits, but destructive range operations always use the entire physical rectangle, **not filtered visible rows**.

## Limitations

Filters are simple "contains" predicates rather than Excel-style filter menus with multiple conditions, dates or numeric comparisons. Range selection is click/Shift-click, not continuous drag, and there is no persistent cell styling, Undo/Redo, or XLSX import parity. The editor's maximum grid is still 10,000 rows × 702 columns.

Automated tests execute the TypeScript selection/filter logic plus native Rust regression tests for replacement boundaries, no-op behavior and formula safety.
