# Koffice Fusion v0.12 — manually refreshed, session-bound links

**This milestone implements a real guarded Sheets → Writer link, not automatic file synchronization.** Its source is the **currently open Sheets workbook in the running Koffice process**. Link tokens are deliberately invalidated when Koffice is restarted or when the Sheets workbook is replaced. After relaunch you must explicitly reconnect the link.

## How to use

1. Open/create a workbook in Sheets and populate a worksheet.
2. Switch to Writer; choose **Link active Sheets** in the Writer toolbar, then name the new embedded table. Writer stores a complete, editable source snapshot in `.kdoc`.
3. The object shows the source workbook and sheet. Click **Check** for link status:
   - `current`: source and embedded snapshot agree.
   - `changed`: source workbook changed; manual refresh is possible.
   - `conflict`: someone edited the embedded copy locally. Refresh is **blocked**.
   - `disconnected`: source session was replaced/restarted or the sheet renamed/deleted.
4. Click **Refresh** to copy the currently active source sheet over an unchanged embedded copy. The document revision increments; **Save Writer** to persist the result.
5. After restarting or opening a different workbook, select the intended sheet in Sheets and click **Reconnect**. Reconnection requires explicit approval and refuses locally changed embedded data.
6. Click **Detach** to preserve the complete embedded workbook, formulas and values as an independent copy.

## Correctness

- A unique session generation token prevents silent rebinding to a new/opened workbook with the same name.
- A link records source workbook title, source sheet name and revision, and the embedded revision last synchronized.
- Refresh checks the exact source session, title and worksheet name, and **refuses to overwrite local edits**.
- No background filesystem access or network sync.
- Legacy `.kdoc` objects omit the optional `link` property and remain compatible.
- Copy/validate/commit operations keep document state unchanged on failures.
- Unit tests cover status, changed-source refresh, formula integrity, conflict, rollback, detached data, disconnected/reconnected sessions and JSON migration.

## Limitations

This is **session-scoped**, manually refreshed data exchange. It does not reconnect a file path automatically, and does not track a workbook after restart without your confirmation. There is no automatic refresh, multi-user sync, general conflict merge, linked Base reports, or persistent dependency graph yet. For serious workflows, retain the source files and verify their contents before reconnecting.

Koffice's first priorities remain cross-platform operation, native performance, usable editors and trustworthy persistence, rather than AI automation.
