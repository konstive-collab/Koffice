//! Guarded, explicit in-memory session links between Writer and Sheets.
//! Never follows file paths, makes background updates, or overwrites local edits.
use crate::engine::Document;
use crate::fusion::{EmbeddedObject, SheetLink};
use crate::suite::SheetBook;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct LinkStatus {
    pub state: String,
    pub detail: String,
    pub source_sheet: String,
    pub source_title: String,
}

fn source_sheet<'a>(link: &SheetLink, source: &'a SheetBook, token: &str) -> Result<&'a crate::suite::Sheet, String> {
    if token != link.source_token {
        return Err("Source session changed. Use Reconnect to explicitly choose the open workbook again.".into());
    }
    if source.title != link.source_title {
        return Err("Source workbook title differs from the linked origin".into());
    }
    source.sheets.iter().find(|sheet| sheet.name == link.source_sheet)
        .ok_or_else(|| "Source sheet has been renamed or deleted".into())
}

pub fn create_link(
    doc: &mut Document, expected_revision: u64, label: String,
    source: &SheetBook, source_index: usize, token: &str,
) -> Result<(), String> {
    if token.is_empty() { return Err("No active workbook session".into()); }
    source.validate()?;
    let tab = source.sheets.get(source_index).ok_or("Source sheet not found")?;
    let mut next=doc.clone();
    next.embed_sheet(expected_revision,label,source,source_index)?;
    *next.embedded_objects.last_mut().ok_or("Embedded sheet missing")?.link_mut() = Some(SheetLink{
        source_token:token.to_string(),
        source_title:source.title.clone(),
        source_sheet:tab.name.clone(),
        source_revision:source.revision,
        clean_embedded_revision:0,
    });
    next.validate()?;
    *doc=next;
    Ok(())
}

pub fn status(doc: &Document, id: &str, source: &SheetBook, token: &str) -> Result<LinkStatus,String> {
    let object=doc.embedded_objects.iter().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    let link=object.link().ok_or("Embedded sheet is not linked")?;
    let info=|state:&str,detail:&str|LinkStatus {
        state:state.into(),detail:detail.into(),
        source_sheet:link.source_sheet.clone(),
        source_title:link.source_title.clone(),
    };
    if object.workbook().revision != link.clean_embedded_revision {
        return Ok(info("conflict","The embedded sheet has local edits. Refresh is blocked to protect them."));
    }
    let tab=match source_sheet(link,source,token) {
        Ok(tab)=>tab,Err(_)=>return Ok(info("disconnected","The original source session or sheet is not available. Reconnect explicitly.")),
    };
    if source.revision==link.source_revision && &object.workbook().sheets[0].cells == &tab.cells {
        return Ok(info("current","The embedded copy matches its last known source revision."));
    }
    Ok(info("changed","Source data or workbook revision changed; use Refresh to copy the selected worksheet."))
}

pub fn refresh(doc: &mut Document, expected_revision: u64, id: &str, source: &SheetBook, token: &str) -> Result<(),String> {
    if doc.revision != expected_revision{return Err("Writer revision conflict".into());}
    let mut next=doc.clone();
    let object=next.embedded_objects.iter_mut().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    let link=object.link().ok_or("Embedded sheet is not linked")?.clone();
    if object.workbook().revision != link.clean_embedded_revision {
        return Err("Local embedded edits detected. Detach the link to keep them; refresh refused.".into());
    }
    let tab=source_sheet(&link,source,token)?.clone();
    let workbook=object.workbook_mut();
    if source.revision == link.source_revision && workbook.sheets[0].cells == tab.cells {
        return Ok(());
    }
    workbook.sheets[0]=tab;
    workbook.revision=workbook.revision.checked_add(1).ok_or("Embedded revision overflow")?;
    let next_clean_rev=workbook.revision;
    let updated=object.link_mut().as_mut().ok_or("Link missing")?;
    updated.source_revision=source.revision;
    updated.clean_embedded_revision=next_clean_rev;
    next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
    next.validate()?;
    *doc=next;
    Ok(())
}

/// Explicit reconnection after restart, new/open, or renamed workbook.
/// Never silently replaces a locally modified sheet.
pub fn reconnect(doc: &mut Document, expected_revision: u64, id: &str, source: &SheetBook, selected: usize, token: &str) -> Result<(),String> {
    if doc.revision != expected_revision {return Err("Writer revision conflict".into());}
    source.validate()?;
    let tab=source.sheets.get(selected).ok_or("Source sheet not found")?.clone();
    let mut next=doc.clone();
    let obj=next.embedded_objects.iter_mut().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    let current=obj.link().ok_or("Embedded sheet is not linked")?.clone();
    if obj.workbook().revision != current.clean_embedded_revision {
        return Err("Local edits prevent reconnection. Detach to keep them.".into());
    }
    let workbook=obj.workbook_mut();
    workbook.sheets[0]=tab.clone();
    workbook.revision=workbook.revision.checked_add(1).ok_or("Embedded revision overflow")?;
    let clean=workbook.revision;
    *obj.link_mut()=Some(SheetLink {
        source_token:token.into(),source_title:source.title.clone(),
        source_sheet:tab.name,source_revision:source.revision,clean_embedded_revision:clean,
    });
    next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
    next.validate()?;
    *doc=next;
    Ok(())
}

pub fn detach(doc: &mut Document, expected_revision: u64, id: &str) -> Result<(),String> {
    if doc.revision != expected_revision { return Err("Writer revision conflict".into()); }
    let mut next=doc.clone();
    let obj=next.embedded_objects.iter_mut().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    if obj.link().is_none(){return Err("Embedded sheet is already independent".into());}
    *obj.link_mut()=None;
    next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
    next.validate()?;
    *doc=next;Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn fixture()->(Document,SheetBook,String){
        let mut source=SheetBook::blank();
        source.put(0,0,"A1".into(),"10".into()).unwrap();
        let mut doc=Document::blank();
        create_link(&mut doc,0,"Sales".into(),&source,0,"session-one").unwrap();
        let id=doc.embedded_objects[0].id().to_string();
        (doc,source,id)
    }
    #[test] fn detects_source_changes_and_refreshes_formulas_without_mutating_source(){
        let (mut doc,mut source,id)=fixture();
        assert_eq!(status(&doc,&id,&source,"session-one").unwrap().state,"current");
        source.put(1,0,"B1".into(),"=SUM(A1:A1)".into()).unwrap();
        assert_eq!(status(&doc,&id,&source,"session-one").unwrap().state,"changed");
        refresh(&mut doc,1,&id,&source,"session-one").unwrap();
        assert_eq!(doc.embedded_objects[0].workbook().value(0,"B1").unwrap(),"10");
        assert_eq!(doc.revision,2);
        assert_eq!(status(&doc,&id,&source,"session-one").unwrap().state,"current");
        assert_eq!(source.sheets[0].cells["B1"],"=SUM(A1:A1)");
    }
    #[test] fn local_edits_block_refresh_and_allow_detach(){
        let (mut doc,mut source,id)=fixture();
        let mut edits=BTreeMap::new();edits.insert("A1".into(),"99".into());
        doc.embedded_put(1,&id,edits).unwrap();
        source.put(1,0,"A1".into(),"25".into()).unwrap();
        assert_eq!(status(&doc,&id,&source,"session-one").unwrap().state,"conflict");
        assert!(refresh(&mut doc,2,&id,&source,"session-one").is_err());
        assert_eq!(doc.embedded_objects[0].workbook().sheets[0].cells["A1"],"99");
        detach(&mut doc,2,&id).unwrap();
        assert!(doc.embedded_objects[0].link().is_none());
        assert_eq!(doc.embedded_objects[0].workbook().sheets[0].cells["A1"],"99");
    }
    #[test] fn old_session_never_silently_relinks_and_explicit_reconnect_works(){
        let (mut doc,source,id)=fixture();
        assert_eq!(status(&doc,&id,&source,"different-session").unwrap().state,"disconnected");
        assert!(refresh(&mut doc,1,&id,&source,"different-session").is_err());
        reconnect(&mut doc,1,&id,&source,0,"different-session").unwrap();
        assert_eq!(status(&doc,&id,&source,"different-session").unwrap().state,"current");
    }
    #[test] fn serialization_preserves_link_metadata_and_legacy_embeds(){
        let (doc,_source,_id)=fixture();
        let json=serde_json::to_string(&doc).unwrap();
        let roundtrip:Document=serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip,doc);
        let old=r#"{"kind":"sheet","id":"obj-legacy","label":"Old","workbook":{"format":"koffice.sheets.v1","title":"Old","revision":0,"sheets":[{"name":"Sheet 1","cells":{}}]}}"#;
        let parsed:EmbeddedObject=serde_json::from_str(old).unwrap();
        assert!(parsed.link().is_none());
        parsed.validate().unwrap();
    }
}
