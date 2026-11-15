//! Local AI/automation command kernel. No network listener or autonomous file permissions.
//! Requests are validated and applied to cloned workspaces, then committed together.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::{engine::{Document, Edit}, suite::{Deck, SheetBook, Slide}};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationBatch {
    pub operations: Vec<AutomationOp>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AutomationOp {
    WriterEdit { expected_revision: u64, edit: Edit },
    SheetsPutBatch { expected_revision: u64, sheet: usize, updates: BTreeMap<String, String> },
    SheetsAddSheet { expected_revision: u64, name: String },
    SheetsRenameSheet { expected_revision: u64, index: usize, name: String },
    SlidesAdd { expected_revision: u64 },
    SlidesUpdate { expected_revision: u64, index: usize, slide: Slide },
    SlidesDuplicate { expected_revision: u64, index: usize },
}

#[derive(Clone, Debug, Serialize)]
pub struct AutomationSnapshot {
    pub writer: Document,
    pub sheets: SheetBook,
    pub slides: Deck,
}

#[derive(Clone, Debug, Serialize)]
pub struct AutomationResult {
    pub applied: usize,
    pub writer_revision: u64,
    pub sheets_revision: u64,
    pub slides_revision: u64,
}

/// All-or-nothing update of the three in-memory editors. Each op uses its
/// current expected revision, so an agent must plan subsequent revisions.
/// The caller must serialize other workspace writes while this executes.
pub fn execute(
    writer: &mut Document,
    sheets: &mut SheetBook,
    slides: &mut Deck,
    batch: AutomationBatch,
) -> Result<AutomationResult, String> {
    if batch.operations.is_empty() || batch.operations.len() > 50 {
        return Err("Automation batch must contain 1–50 operations".into());
    }
    let mut w = writer.clone();
    let mut s = sheets.clone();
    let mut d = slides.clone();
    let operation_count = batch.operations.len();
    for (index, op) in batch.operations.into_iter().enumerate() {
        let outcome = match op {
            AutomationOp::WriterEdit { expected_revision, edit } => w.apply(expected_revision, edit),
            AutomationOp::SheetsPutBatch { expected_revision, sheet, updates } => s.put_batch(expected_revision, sheet, updates),
            AutomationOp::SheetsAddSheet { expected_revision, name } => s.add_sheet(expected_revision, name),
            AutomationOp::SheetsRenameSheet { expected_revision, index, name } => s.rename_sheet(expected_revision, index, name),
            AutomationOp::SlidesAdd { expected_revision } => d.add(expected_revision),
            AutomationOp::SlidesUpdate { expected_revision, index, slide } => d.update(expected_revision, index, slide),
            AutomationOp::SlidesDuplicate { expected_revision, index } => d.duplicate(expected_revision, index),
        };
        outcome.map_err(|e| format!("Operation {} rejected: {e}; no changes committed", index + 1))?;
    }
    w.validate()?;
    s.validate()?;
    d.validate()?;
    let result = AutomationResult {
        applied: operation_count,
        writer_revision: w.revision,
        sheets_revision: s.revision,
        slides_revision: d.revision,
    };
    *writer = w;
    *sheets = s;
    *slides = d;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn workspace() -> (Document, SheetBook, Deck) {
        (Document::blank(), SheetBook::blank(), Deck::blank())
    }
    #[test]
    fn cross_editor_commit_and_revisions() {
        let (mut w, mut s, mut d) = workspace();
        let batch: AutomationBatch = serde_json::from_str(r#"{"operations":[
          {"type":"writer_edit","expected_revision":0,"edit":{"type":"replace_all","text":"Привет ☕"}},
          {"type":"sheets_put_batch","expected_revision":0,"sheet":0,"updates":{"A1":"4","A2":"6"}},
          {"type":"slides_add","expected_revision":0},
          {"type":"sheets_add_sheet","expected_revision":1,"name":"Reports"}
        ]}"#).unwrap();
        let r=execute(&mut w,&mut s,&mut d,batch).unwrap();
        assert_eq!(r.applied,4);
        assert_eq!((w.revision,s.revision,d.revision),(1,2,1));
        assert_eq!(w.text,"Привет ☕");
        assert_eq!(s.sheets[1].name,"Reports");
    }
    #[test]
    fn failed_operation_rolls_back_every_editor() {
        let (mut w, mut s, mut d)=workspace();
        let batch: AutomationBatch=serde_json::from_str(r#"{"operations":[
          {"type":"writer_edit","expected_revision":0,"edit":{"type":"replace_all","text":"should not survive"}},
          {"type":"sheets_put_batch","expected_revision":0,"sheet":0,"updates":{"A0":"invalid"}}
        ]}"#).unwrap();
        assert!(execute(&mut w,&mut s,&mut d,batch).is_err());
        assert_eq!(w,Document::blank());
        assert_eq!(s,SheetBook::blank());
        assert_eq!(d,Deck::blank());
    }
    #[test]
    fn rejects_stale_revision_and_unknown_actions() {
        let (mut w,mut s,mut d)=workspace();
        let batch:AutomationBatch=serde_json::from_str(r#"{"operations":[{"type":"slides_add","expected_revision":9}]}"#).unwrap();
        assert!(execute(&mut w,&mut s,&mut d,batch).is_err());
        assert!(serde_json::from_str::<AutomationBatch>(r#"{"operations":[{"type":"erase_disk"}]}"#).is_err());
        assert!(serde_json::from_str::<AutomationBatch>(r#"{"operations":[{"type":"slides_add","expected_revision":0,"path":"/tmp"}]}"#).is_err());
    }
    #[test]
    fn rejects_empty_and_oversized_batch() {
        let (mut w,mut s,mut d)=workspace();
        assert!(execute(&mut w,&mut s,&mut d,AutomationBatch{operations:vec![]}).is_err());
        let operations=(0..51).map(|i|AutomationOp::SlidesAdd{expected_revision:i}).collect();
        assert!(execute(&mut w,&mut s,&mut d,AutomationBatch{operations}).is_err());
    }
}