# Kodi Writer 0.21 — measured multi-page preview

This release adds **Pages** to Writer. It is a separate read-only view that lays out the current document on physical A4/Letter/Legal pages using the browser's actual text and block measurements.

The preview uses the selected page size, margins, orientation and header/footer. Navigate with Previous/Next or Page Up/Page Down, choose 50–100% zoom, and return to the editor with Edit document or Escape. Manual page breaks create new pages.

Crucially, the page view **does not modify or reparent the live contenteditable editor**. It reads the editor's current unsaved markup, sanitizes it and creates independent copies for measurement and display. Cursor position, selection, and unsaved content remain in the original editor. A hidden measuring page uses the same CSS for font metrics and page dimensions as the visible pages.

## Integrity limits

Only 1,500 top-level blocks or 250 pages are laid out in one preview; exceeding the limit produces a visible warning, never a change to the original document. A table or paragraph larger than a single page remains scrollable and is flagged as oversized. It is not automatically split into smaller pieces, and no text is silently discarded from the native `.kdoc`.

## Current limitations

This is the **first measured page preview**, not yet a fully integrated WYSIWYG multi-page editing engine. Editing continues on a continuous document surface. Large paragraphs/tables cannot yet be split precisely across page boundaries, and print pagination may differ from the preview because of platform WebView/print drivers. Complex inline styles and floating images remain unsupported. Future milestones include line-level splits, widow/orphan constraints, anchored objects, page numbering, and stable cursor/navigation through editable pages.
