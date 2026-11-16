# Kodi Writer 0.19 — Rich text foundations

Koffice Writer now stores a **sanitized rich-text fragment** in native `.kdoc` in addition to the plain-text representation used by the earlier engine and automation interface.

## Editing

The Writer page uses a real editable document surface. Select text and use Bold, Italic, Underline, Strike, Heading 1, Heading 2, Body, Quote, bullet or numbered list. Insert a basic 2-column editable table, horizontal rule or current date. The toolbar exposes Undo/Redo for editor actions; browser-native Ctrl+Z/Ctrl+Y also works while the editor has focus. This is local editor undo, **not** yet a cross-document persistent history.

To export, choose HTML in Writer's Export menu or print. Both use sanitized document markup rather than Markdown markers. Existing Fusion embedded Sheets tables still appear after the rich-text document and retain their native formula model.

## File compatibility and safety

The native `koffice.writer.v1` JSON format gains an optional `rich_html` field. Old documents without it open as paragraphs derived from `text`. When a legacy plain-text API/automation edit changes the document, stale rich formatting is discarded so offsets cannot silently attach to unrelated characters.

HTML is restricted to basic text structure, tables and inline emphasis. Unsupported elements are removed when opened, and all HTML attributes (including event handlers, styles, links and image sources) are dropped before displaying or exporting. The Rust layer rejects obvious active content and limits markup size to 4 MiB, with the document's existing 32 MiB total size bound.

## Known limitations

This is a **rich-text foundation**, not yet a full Word-class engine. It does not implement DOCX or ODT interoperability, advanced pagination, embedded image files, anchored floating objects, columns, headers and footers, tracked changes, templates, print-layout fidelity, background spellchecking or document-wide undo. Content is shown on one scrollable paper page and grows as necessary.

The embedded table support remains separate from the rich body; it is not yet inline anchored to arbitrary cursor positions. The current editor relies on standard WebView editing commands (which are not identical across every operating system); hands-on cross-platform QA remains important.
