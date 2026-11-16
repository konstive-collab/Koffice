//! Koffice Fusion: self-contained, typed cross-editor objects.
//! v0.10 supports embedded Sheets copies in Writer. Linked data sources are future work.
use crate::suite::SheetBook;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_OBJECTS: usize = 12;
const MAX_EMBEDDED_CELLS: usize = 25_000;
static OBJECT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EmbeddedObject {
    Sheet { id: String, label: String, workbook: SheetBook },
}

impl EmbeddedObject {
    pub fn id(&self) -> &str { match self { Self::Sheet { id, .. } => id } }
    pub fn workbook(&self) -> &SheetBook { match self { Self::Sheet { workbook, .. } => workbook } }
    pub fn workbook_mut(&mut self) -> &mut SheetBook { match self { Self::Sheet { workbook, .. } => workbook } }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Sheet { id, label, workbook } => {
                if id.is_empty() || id.len() > 96 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err("Invalid embedded object identifier".into());
                }
                if label.trim().is_empty() || label.chars().count() > 120 {
                    return Err("Embedded object label must contain 1–120 characters".into());
                }
                workbook.validate()?;
                if workbook.sheets.len() != 1 { return Err("Embedded sheet must contain exactly one worksheet".into()); }
                Ok(())
            }
        }
    }
}

pub fn validate_objects(objects: &[EmbeddedObject]) -> Result<(), String> {
    if objects.len() > MAX_OBJECTS { return Err("Document exceeds 12 embedded objects".into()); }
    let mut ids = BTreeSet::new();
    let mut total_cells = 0usize;
    for object in objects {
        object.validate()?;
        if !ids.insert(object.id()) { return Err("Duplicate embedded object identifier".into()); }
        total_cells = total_cells.checked_add(object.workbook().sheets[0].cells.len())
            .ok_or("Embedded cell count overflow")?;
        if total_cells > MAX_EMBEDDED_CELLS { return Err("Document exceeds 25,000 embedded cells".into()); }
    }
    Ok(())
}

/// Best-effort process-local unique IDs for document-local objects, not security tokens.
fn fresh_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("obj-{}-{nanos}-{}", std::process::id(), OBJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed))
}

pub fn copy_sheet(label: String, source: &SheetBook, sheet_index: usize) -> Result<EmbeddedObject, String> {
    source.validate()?;
    let tab = source.sheets.get(sheet_index).ok_or("Source sheet not found")?.clone();
    let book = SheetBook { format: source.format.clone(), title: label.clone(), revision: 0, sheets: vec![tab] };
    let object = EmbeddedObject::Sheet { id: fresh_id(), label, workbook: book };
    object.validate()?;
    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_full_sheet_without_linking_source() {
        let mut source=SheetBook::blank();
        source.put(0,0,"A1".into(),"14".into()).unwrap();
        source.put(1,0,"B1".into(),"=SUM(A1:A1)".into()).unwrap();
        let object=copy_sheet("Budget".into(),&source,0).unwrap();
        assert_eq!(object.workbook().sheets[0].cells["B1"],"=SUM(A1:A1)");
        assert_eq!(object.workbook().value(0,"B1").unwrap(),"14");
        assert_eq!(object.workbook().revision,0);
        assert_eq!(source.sheets[0].cells["A1"],"14");
    }
    #[test]
    fn rejects_duplicate_identifiers() {
        let x=copy_sheet("Example".into(),&SheetBook::blank(),0).unwrap();
        assert!(validate_objects(&[x.clone(),x]).is_err());
    }
}
