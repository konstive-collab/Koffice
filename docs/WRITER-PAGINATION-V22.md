# Koffice 0.22 — paragraph and table-row pagination

The Writer **Pages** command now uses a more capable read-only layout engine. It measures content against the actual page area, splits long ordinary paragraphs across pages using binary search, and clones the correct DOM range to retain nested inline formatting (bold, italic, underline, etc.). The algorithm avoids breaking UTF-16 surrogate pairs when calculating offsets.

Tables are laid out **one row at a time** instead of being treated as single indivisible blocks. Rows move to the next page when they no longer fit; where a table has a THEAD, it is repeated in subsequent table fragments. A single enormous row still cannot be split without loss of table structure, so the UI warns and makes the page scrollable instead.

The preview never mutates the source editor, document text, native file or saved revision. It copies sanitized rich content and lays out at most 1,500 top-level blocks and 250 pages in a single preview. Overflow and safety limits are visibly reported, rather than silently discarding data from `.kdoc`.

**Limits:** this is not yet in-place editing across separate pages. Complex lists, deeply nested blocks and mixed content may still move together; line-level splitting uses DOM text offsets, not full Unicode grapheme clusters. Font substitutions and native printer pagination may differ across machines. A fully paginated editable canvas, stable cursor position across page transitions, floating objects and precise DOCX interoperability remain future engineering work.
