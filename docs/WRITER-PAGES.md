# Kodi Writer v0.20 — page setup, margins and explicit page breaks

This iteration introduces **persistent physical page settings** in native `.kdoc`. Open **Page setup** in the Writer formatting toolbar to choose A4, US Letter or Legal, portrait/landscape orientation, margins (5–60 mm), and optional header/footer text.

The preview paper uses physical millimeter sizes and CSS variables. Choose **Page break** to insert a manual break marker in the editable rich body, preserved in `.kdoc` and applied as an explicit print/HTML page break. Printed HTML receives matching `@page` size and margin settings. The entered header/footer text is escaped before rendering; neither supports executable markup.

### Compatibility and protection

Old documents without a `page` property receive A4 portrait, 20 mm margins and blank headers/footers. Page changes use a Rust clone/validate/commit transaction and reject invalid margins, unsupported paper types, stale revisions and oversized header/footer text. Settings persist through save/reopen and do not reset Fusion embedded worksheets or rich-text data.

### Limitations

**This does not yet implement automatic page-by-page layout**: long documents still use one growing content editor, with CSS page breaks at manual markers and print engine pagination. Page numbers, repeating headers/footers on every printed page and exact WYSIWYG print pagination vary by WebView and are not guaranteed. Floating images, DOCX interoperability, PDF fidelity, section-specific page setup, advanced layout and genuine print preview remain future milestones. Existing Undo/Redo remains editor-local, not a persistent document history.
