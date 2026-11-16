//! File format and editing kernel. GUI and future AI adapters must call this layer.
use serde::{Deserialize, Serialize};
use crate::fusion::{self, EmbeddedObject};
use std::collections::BTreeMap;
use std::{fs, io::Write, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

pub const FORMAT: &str = "koffice.writer.v1";
const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_CHARS: usize = 8_000_000;
static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub title: String,
    pub text: String,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embedded_objects: Vec<EmbeddedObject>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    ReplaceAll { text: String },
    Insert { at: usize, text: String },
    Delete { start: usize, end: usize },
}

impl Document {
    pub fn blank() -> Self {
        Self { format: FORMAT.into(), title: "Untitled".into(), text: String::new(), revision: 0, embedded_objects: vec![] }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT { return Err("Unsupported Koffice document format".into()); }
        if self.title.trim().is_empty() || self.title.chars().count() > 200 { return Err("Document title must contain 1–200 characters".into()); }
        if self.text.chars().count() > MAX_CHARS { return Err("Document exceeds the maximum character limit".into()); }
        fusion::validate_objects(&self.embedded_objects)?;
        Ok(())
    }
    /// Atomic-in-memory edit. Unicode offsets are character indices, never byte offsets.
    pub fn apply(&mut self, expected_revision: u64, edit: Edit) -> Result<(), String> {
        if self.revision != expected_revision { return Err("Revision conflict: reload the document before editing".into()); }
        let mut candidate = self.clone();
        match edit {
            Edit::ReplaceAll { text } => candidate.text = text,
            Edit::Insert { at, text } => {
                let pos = char_byte_offset(&candidate.text, at)?;
                candidate.text.insert_str(pos, &text);
            }
            Edit::Delete { start, end } => {
                if end < start { return Err("Invalid deletion range".into()); }
                let begin = char_byte_offset(&candidate.text, start)?;
                let finish = char_byte_offset(&candidate.text, end)?;
                candidate.text.replace_range(begin..finish, "");
            }
        }
        candidate.revision = candidate.revision.checked_add(1).ok_or("Revision overflow")?;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Create an independent embedded copy of a complete worksheet.
    pub fn embed_sheet(&mut self, expected_revision: u64, label: String, source: &crate::suite::SheetBook, sheet: usize) -> Result<(), String> {
        self.check_revision(expected_revision)?;
        let mut next = self.clone();
        next.embedded_objects.push(fusion::copy_sheet(label, source, sheet)?);
        next.bump()?;
        next.validate()?;
        *self = next;
        Ok(())
    }
    /// Edit an embedded worksheet using the exact Sheets engine and its formula rules.
    /// Both workbook and document revision increments commit together or not at all.
    pub fn embedded_put(&mut self, expected_revision: u64, object_id: &str, updates: BTreeMap<String, String>) -> Result<(), String> {
        self.check_revision(expected_revision)?;
        let mut next = self.clone();
        let object = next.embedded_objects.iter_mut().find(|x| x.id() == object_id).ok_or("Embedded object not found")?;
        let current = object.workbook().revision;
        object.workbook_mut().put_batch(current, 0, updates)?;
        next.bump()?;
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn remove_embedded(&mut self, expected_revision: u64, object_id: &str) -> Result<(), String> {
        self.check_revision(expected_revision)?;
        let mut next = self.clone();
        let before = next.embedded_objects.len();
        next.embedded_objects.retain(|x| x.id() != object_id);
        if next.embedded_objects.len() == before { return Err("Embedded object not found".into()); }
        next.bump()?;
        next.validate()?;
        *self = next;
        Ok(())
    }
    fn check_revision(&self, expected: u64) -> Result<(), String> {
        if self.revision != expected { Err("Revision conflict".into()) } else { Ok(()) }
    }
    fn bump(&mut self) -> Result<(), String> {
        self.revision = self.revision.checked_add(1).ok_or("Revision overflow")?;
        Ok(())
    }
}
fn char_byte_offset(text: &str, index: usize) -> Result<usize, String> {
    if index == text.chars().count() { return Ok(text.len()); }
    text.char_indices().nth(index).map(|(i, _)| i).ok_or_else(|| "Character index out of bounds".into())
}

pub fn open(path: &Path) -> Result<Document, String> {
    let meta = fs::metadata(path).map_err(|e| format!("Cannot read file: {e}"))?;
    if meta.len() > MAX_DOCUMENT_BYTES { return Err("Document is too large".into()); }
    let contents = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let doc: Document = serde_json::from_slice(&contents).map_err(|e| format!("Invalid Koffice document: {e}"))?;
    doc.validate()?;
    Ok(doc)
}

/// Writes to a sibling temporary file and renames over the target.
/// File replacement is atomic on supported same-filesystem configurations.
pub fn save(path: &Path, doc: &Document) -> Result<(), String> {
    doc.validate()?;
    let serialized = serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?;
    save_bytes(path, &serialized)
}

pub fn save_bytes(path: &Path, serialized: &[u8]) -> Result<(), String> {
    if serialized.len() as u64 > MAX_DOCUMENT_BYTES { return Err("Serialized document is too large".into()); }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    if !parent.is_dir() { return Err("Destination directory does not exist".into()); }
    let name = path.file_name().ok_or("Invalid destination filename")?;
    let seq = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(".{}.{}.{}.tmp", name.to_string_lossy(), std::process::id(), seq);
    let temporary: PathBuf = parent.join(temp_name);
    let result = (|| -> Result<(), String> {
        let mut f = fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e| format!("Cannot create temporary file: {e}"))?;
        f.write_all(&serialized).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        if path.exists() {
            // Windows rename cannot replace an existing destination. Keep a recoverable
            // backup while swapping and roll back if the final rename fails.
            let backup = parent.join(format!(".{}.{}.{}.bak", name.to_string_lossy(), std::process::id(), seq));
            fs::rename(path, &backup).map_err(|e| format!("Cannot back up the existing document: {e}"))?;
            if let Err(e) = fs::rename(&temporary, path) {
                let _ = fs::rename(&backup, path);
                return Err(format!("Cannot replace document; recovery backup at {}: {e}", backup.display()));
            }
            let _ = fs::remove_file(backup);
        } else {
            fs::rename(&temporary, path).map_err(|e| format!("Cannot finalize save: {e}"))?;
        }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn unicode_edit_offsets() {
        let mut d = Document::blank();
        d.apply(0, Edit::ReplaceAll { text: "A☕Я".into() }).unwrap();
        d.apply(1, Edit::Insert { at: 2, text: "!".into() }).unwrap();
        assert_eq!(d.text, "A☕!Я");
        d.apply(2, Edit::Delete { start: 1, end: 3 }).unwrap();
        assert_eq!(d.text, "AЯ");
    }
    #[test] fn rejects_stale_and_invalid_edits_without_mutation() {
        let mut d = Document::blank();
        assert!(d.apply(1, Edit::Insert { at: 0, text: "bad".into() }).is_err());
        assert!(d.apply(0, Edit::Delete { start: 0, end: 2 }).is_err());
        assert_eq!(d, Document::blank());
    }
    #[test] fn rejects_invalid_format() {
        let mut d = Document::blank();
        d.format = "other".into();
        assert!(d.validate().is_err());
    }

    #[test]
    fn embedded_sheet_roundtrip_preserves_formulas_and_legacy_documents() {
        let mut source=crate::suite::SheetBook::blank();
        source.put(0,0,"A1".into(),"8".into()).unwrap();
        source.put(1,0,"A2".into(),"4".into()).unwrap();
        source.put(2,0,"B1".into(),"=SUM(A1:A2)".into()).unwrap();
        let mut doc=Document::blank();
        doc.embed_sheet(0,"Results".into(),&source,0).unwrap();
        let bytes=serde_json::to_vec(&doc).unwrap();
        let recovered:Document=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(recovered,doc);
        assert_eq!(recovered.embedded_objects[0].workbook().value(0,"B1").unwrap(),"12");

        // Reopen the actual .kdoc file: no sidecar .ksheet is needed.
        let path=std::env::temp_dir().join(format!("fusion-{}-{}.kdoc",std::process::id(),SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        save(&path,&doc).unwrap();
        let disk=open(&path).unwrap();
        assert_eq!(disk.embedded_objects[0].workbook().value(0,"B1").unwrap(),"12");
        assert_eq!(disk,doc);
        std::fs::remove_file(path).unwrap();
        // Legacy kdoc did not have embedded_objects: it must still open.
        let old=r#"{"format":"koffice.writer.v1","title":"Old","text":"hello","revision":0}"#;
        let legacy:Document=serde_json::from_str(old).unwrap();
        assert!(legacy.embedded_objects.is_empty());
        assert!(legacy.validate().is_ok());
    }
    #[test]
    fn embedded_edits_are_atomic_and_do_not_mutate_original_workbook() {
        let mut source=crate::suite::SheetBook::blank();
        source.put(0,0,"A1".into(),"10".into()).unwrap();
        let mut doc=Document::blank();
        doc.embed_sheet(0,"Table".into(),&source,0).unwrap();
        let id=doc.embedded_objects[0].id().to_string();
        let mut invalid=BTreeMap::new();invalid.insert("A1".into(),"99".into());invalid.insert("A0".into(),"bad".into());
        assert!(doc.embedded_put(1,&id,invalid).is_err());
        assert_eq!(doc.revision,1);
        assert_eq!(doc.embedded_objects[0].workbook().sheets[0].cells["A1"],"10");
        let mut valid=BTreeMap::new();valid.insert("A1".into(),"15".into());
        doc.embedded_put(1,&id,valid).unwrap();
        assert_eq!(doc.revision,2);
        assert_eq!(doc.embedded_objects[0].workbook().revision,1);
        assert_eq!(source.sheets[0].cells["A1"],"10");
        assert!(doc.remove_embedded(1,&id).is_err());
        doc.remove_embedded(2,&id).unwrap();
        assert!(doc.embedded_objects.is_empty());
    }
    #[test] fn save_and_reopen() {
        let mut path = std::env::temp_dir();
        path.push(format!("koffice-test-{}-{}.kdoc", std::process::id(), SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        let mut d = Document::blank();
        d.title = "Пример".into();
        d.apply(0, Edit::ReplaceAll { text: "Привет ☕".into() }).unwrap();
        save(&path, &d).unwrap();
        assert_eq!(open(&path).unwrap(), d);
        save(&path, &d).unwrap();
        assert_eq!(open(&path).unwrap(), d);
        std::fs::remove_file(path).unwrap();
    }
}