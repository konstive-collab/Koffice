//! Koffice Fusion transfer commands: lossless embedded-sheet extraction and
//! intentionally static Base-to-Slides snapshots. Both use normal document engines.
use crate::{engine::Document, suite::{Deck, SheetBook, Slide}, base::QueryResult};

/// Appends a full embedded worksheet to an existing workbook without replacing
/// any other worksheet. Formula strings and populated cells stay byte-for-byte
/// identical. Revision check + validate ensures atomic commit.
pub fn append_embedded_to_workbook(
    document: &Document, object_id: &str, workbook: &mut SheetBook, expected_revision: u64,
) -> Result<(), String> {
    document.validate()?;
    if workbook.revision != expected_revision { return Err("Workbook revision conflict".into()); }
    let object = document.embedded_objects.iter().find(|o| o.id() == object_id)
        .ok_or("Embedded object not found")?;
    let source = object.workbook().sheets.first().ok_or("Embedded worksheet missing")?;
    if workbook.sheets.len() >= 50 { return Err("Workbook already contains 50 sheets".into()); }

    let base = source.name.trim();
    let mut name = base.to_string();
    let mut suffix = 2;
    while workbook.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(&name)) {
        let addition = format!(" ({suffix})");
        let keep = 200usize.saturating_sub(addition.chars().count());
        let prefix: String = base.chars().take(keep).collect();
        name = format!("{prefix}{addition}");
        suffix += 1;
        if suffix > 100 { return Err("Cannot find a unique sheet name".into()); }
    }

    let mut candidate = workbook.clone();
    let mut sheet = source.clone();
    sheet.name = name;
    candidate.sheets.push(sheet);
    candidate.revision = candidate.revision.checked_add(1).ok_or("Workbook revision overflow")?;
    candidate.validate()?;
    *workbook = candidate;
    Ok(())
}

/// Append an embedded slide worksheet to standalone Sheets, retaining formulas.
/// No other workbook tab is changed; duplicate names are deterministically renamed.
pub fn append_slide_embedded_to_workbook(
    deck:&Deck,slide_index:usize,object_id:&str,book:&mut SheetBook,expected:u64
)->Result<(),String>{
    deck.validate()?;
    if book.revision!=expected {return Err("Workbook revision conflict".into());}
    let slide=deck.slides.get(slide_index).ok_or("Slide not found")?;
    let object=slide.embedded_objects.iter().find(|o|o.id()==object_id).ok_or("Embedded object not found")?;
    if book.sheets.len()>=50 {return Err("Workbook already has 50 sheets".into());}
    let mut next=book.clone();
    let mut tab=object.workbook().sheets[0].clone();
    let original=tab.name.clone();
    let mut candidate=original.clone();
    let mut suffix=2usize;
    while next.sheets.iter().any(|s|s.name.eq_ignore_ascii_case(&candidate)){
        let tail=format!(" ({suffix})");
        let prefix:String=original.chars().take(200usize.saturating_sub(tail.chars().count())).collect();
        candidate=format!("{prefix}{tail}");
        suffix+=1;
        if suffix>100 {return Err("Cannot find unique worksheet name".into());}
    }
    tab.name=candidate;
    next.sheets.push(tab);
    next.revision=next.revision.checked_add(1).ok_or("Revision overflow")?;
    next.validate()?;*book=next;Ok(())
}

/// Converts the first page of a Base table into a readable slide. This is a
/// labelled snapshot, NOT a live database connection. Excludes fields beyond
/// the UI's sensible limits; never silently claims the entire table was copied.
pub fn base_snapshot_slide(table: &str, query: &QueryResult) -> Result<Slide, String> {
    if table.is_empty() || table.len() > 64 || !table.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err("Invalid table name".into());
    }
    const COLS: usize = 8;
    const ROWS: usize = 20;
    const CELL_CHARS: usize = 72;
    let columns = query.columns.iter().take(COLS).map(|c| c.as_str()).collect::<Vec<_>>();
    if columns.is_empty() { return Err("No columns available for slide".into()); }
    let clean = |value: &str| -> String {
        let text = value.replace('\n', " ").replace('\r', " ").replace('\t', " ");
        let mut chars = text.chars();
        let prefix: String = chars.by_ref().take(CELL_CHARS).collect();
        if chars.next().is_some() {format!("{prefix}…")} else {prefix}
    };
    let mut lines = vec![columns.iter().map(|s|clean(s)).collect::<Vec<_>>().join(" | ")];
    for row in query.rows.iter().take(ROWS) {
        let values = row.iter().take(COLS).map(|v| {
            match v {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(s) => clean(s),
                other => clean(&other.to_string()),
            }
        }).collect::<Vec<_>>();
        lines.push(values.join(" | "));
    }
    let omitted = query.truncated || query.rows.len() > ROWS || query.columns.len() > COLS;
    if omitted { lines.push("[Snapshot truncated: more rows or columns are not shown]".into()); }
    let slide = Slide {
        title: format!("Base: {table}"),
        body: lines.join("\n"),
        notes: format!("Static snapshot from Koffice Base table '{table}'. No live database link. First {} rows, first {} columns displayed.{}", query.rows.len().min(ROWS),columns.len(),if omitted{" Additional source data omitted."}else{""}),
        base_source: None,
        embedded_objects: vec![],
        object_frames: std::collections::BTreeMap::new(),
    };
    if slide.body.len() > 16_384 || slide.notes.len() > 16_384 {return Err("Slide snapshot too large".into());}
    Ok(slide)
}

/// Append the snapshot without overwriting existing slide content.
pub fn append_base_snapshot(
    deck: &mut Deck, expected_revision: u64, table: &str, query: &QueryResult,
) -> Result<(), String> {
    if deck.revision != expected_revision {return Err("Presentation revision conflict".into());}
    let slide = base_snapshot_slide(table, query)?;
    let mut next = deck.clone();
    next.add(expected_revision)?;
    let index = next.slides.len() - 1;
    next.update(next.revision,index,slide)?;
    next.validate()?;
    *deck = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn embedded_sheet_transfer_preserves_formula_and_other_sheets() {
        let mut source=SheetBook::blank();
        source.put(0,0,"A1".into(),"10".into()).unwrap();
        source.put(1,0,"B1".into(),"=SUM(A1:A1)".into()).unwrap();
        let mut doc=Document::blank();
        doc.embed_sheet(0,"Budget".into(),&source,0).unwrap();
        let key=doc.embedded_objects[0].id().to_string();

        let mut target=SheetBook::blank();
        append_embedded_to_workbook(&doc,&key,&mut target,0).unwrap();
        assert_eq!(target.sheets.len(),2);
        assert_eq!(target.sheets[1].name,"Sheet 1 (2)");
        assert_eq!(target.sheets[1].cells["B1"],"=SUM(A1:A1)");
        assert_eq!(target.value(1,"B1").unwrap(),"10");
        assert_eq!(source.sheets[0].cells["A1"],"10");
        assert_eq!(target.revision,1);
        assert!(append_embedded_to_workbook(&doc,&key,&mut target,0).is_err());
        assert_eq!(target.sheets.len(),2);
        append_embedded_to_workbook(&doc,&key,&mut target,1).unwrap();
        assert_eq!(target.sheets[2].name,"Sheet 1 (3)");
    }

    #[test]
    fn failed_extract_rolls_back() {
        let mut doc=Document::blank();
        doc.embed_sheet(0,"Blank".into(),&SheetBook::blank(),0).unwrap();
        let mut book=SheetBook::blank();
        assert!(append_embedded_to_workbook(&doc,"unknown",&mut book,0).is_err());
        assert_eq!(book,SheetBook::blank());
    }

    #[test]
    fn database_snapshot_is_bounded_and_clear_about_truncation() {
        let rows=(0..30).map(|i|vec![serde_json::json!(i),serde_json::json!("A\nB")]).collect();
        let query=QueryResult{columns:vec!["id".into(),"value".into()],rows,truncated:true};
        let mut deck=Deck::blank();
        append_base_snapshot(&mut deck,0,"People",&query).unwrap();
        assert_eq!(deck.slides.len(),2);
        assert!(deck.slides[1].title.contains("People"));
        assert!(deck.slides[1].body.contains("Snapshot truncated"));
        assert!(deck.slides[1].notes.contains("No live database link"));
        assert!(!deck.slides[1].body.contains("A\nB"));
        assert_eq!(deck.revision,2);
        let before=deck.clone();
        assert!(append_base_snapshot(&mut deck,0,"People",&query).is_err());
        assert_eq!(deck,before);
    }

    #[test]
    fn rejects_empty_source_and_invalid_tables() {
        let query=QueryResult{columns:vec![],rows:vec![],truncated:false};
        assert!(base_snapshot_slide("Data",&query).is_err());
        let query=QueryResult{columns:vec!["id".into()],rows:vec![],truncated:false};
        assert!(base_snapshot_slide("bad-name! ",&query).is_err());
    }

    #[test]
    fn nontrivial_values_do_not_corrupt_original_workbook() {
        let mut source=SheetBook::blank();
        let mut updates=BTreeMap::new();
        updates.insert("A1".into(),"=2+3*4".into());
        updates.insert("B5".into(),"Привет ☕".into());
        source.put_batch(0,0,updates).unwrap();
        let mut doc=Document::blank();
        doc.embed_sheet(0,"Unicode".into(),&source,0).unwrap();
        let mut result=SheetBook::blank();
        append_embedded_to_workbook(&doc,doc.embedded_objects[0].id(),&mut result,0).unwrap();
        assert_eq!(result.sheets[1].cells["B5"],"Привет ☕");
        assert_eq!(result.value(1,"A1").unwrap(),"14");
        assert_eq!(source.sheets.len(),1);
    }
}
