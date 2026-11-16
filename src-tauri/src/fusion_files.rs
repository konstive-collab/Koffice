//! File-backed Koffice Fusion links, explicitly checked or refreshed by the user.
//! A document can store a path, but *opening it never follows that path*.
//! All operations are read-only against sources and never overwrite local edits.
use crate::engine::Document;
use crate::fusion::FileSheetLink;
use crate::suite::{self, SheetBook, Sheet};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone,Debug,Serialize,PartialEq)]
pub struct FileLinkStatus {
    pub state: String,
    pub detail: String,
    pub source_path: String,
    pub source_sheet: String,
}

fn canonical_source(path:&str)->Result<PathBuf,String>{
    if path.len()>4096 || !path.ends_with(".ksheet") {return Err("Choose a .ksheet source file".into());}
    let result=std::fs::canonicalize(path).map_err(|e|format!("Cannot locate source: {e}"))?;
    if !result.is_file() {return Err("Source must be a file".into());}
    if result.extension().and_then(|x|x.to_str())!=Some("ksheet") {return Err("Invalid source extension".into());}
    Ok(result)
}
pub fn load_source(path:&str)->Result<(String,SheetBook),String>{
    let path=canonical_source(path)?;
    let source:SheetBook=suite::load_json(&path)?;
    source.validate()?;
    let name=path.to_str().ok_or("Source path is not valid Unicode")?.to_string();
    Ok((name,source))
}
pub fn list_sheets(path:&str)->Result<Vec<String>,String>{
    let (_,source)=load_source(path)?;
    Ok(source.sheets.iter().map(|s|s.name.clone()).collect())
}
fn get_original<'a>(origin:&FileSheetLink,source:&'a SheetBook)->Result<&'a Sheet,String>{
    if source.title!=origin.source_title {return Err("Source title changed: explicit reconnect required".into());}
    source.sheets.iter().find(|s|s.name==origin.source_sheet)
        .ok_or_else(||"Source sheet renamed or deleted: explicit reconnect required".into())
}
fn find_link<'a>(doc:&'a Document,id:&str)->Result<(&'a crate::fusion::EmbeddedObject,&'a FileSheetLink),String>{
    let object=doc.embedded_objects.iter().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    let link=object.file_link().ok_or("Embedded object does not have a file link")?;
    Ok((object,link))
}
fn check_local(object:&crate::fusion::EmbeddedObject,origin:&FileSheetLink)->Result<(),String>{
    if object.workbook().revision!=origin.clean_embedded_revision {
        return Err("Local changes detected. Refresh cannot overwrite edited cells. Detach to keep your changes.".into());
    }
    Ok(())
}
pub fn attach(doc:&mut Document,expected_revision:u64,path:&str,sheet_index:usize,label:String)->Result<(),String>{
    let (canonical,source)=load_source(path)?;
    let tab=source.sheets.get(sheet_index).ok_or("Source sheet index out of range")?;
    let mut next=doc.clone();
    next.embed_sheet(expected_revision,label,&source,sheet_index)?;
    *next.embedded_objects.last_mut().ok_or("Embedded object missing")?.file_link_mut()=Some(FileSheetLink{
        source_path:canonical,source_title:source.title,
        source_sheet:tab.name.clone(),source_revision:source.revision,
        clean_embedded_revision:0,
    });
    next.validate()?;
    *doc=next;
    Ok(())
}
pub fn status(doc:&Document,id:&str)->Result<FileLinkStatus,String>{
    let (object,origin)=find_link(doc,id)?;
    let create=|state:&str,detail:&str|FileLinkStatus{
        state:state.into(),detail:detail.into(),
        source_path:origin.source_path.clone(),source_sheet:origin.source_sheet.clone(),
    };
    if check_local(object,origin).is_err(){
        return Ok(create("conflict","Embedded cells have been edited locally; refresh is blocked."));
    }
    // Read path only because the user invoked the status command.
    let source=match load_source(&origin.source_path){
        Ok((_,book))=>book,Err(_)=>return Ok(create("disconnected","Cannot load the source file. Use Reconnect to select a file.")),
    };
    let sheet=match get_original(origin,&source){
        Ok(x)=>x,Err(_)=>return Ok(create("disconnected","Source title or worksheet changed. Use Reconnect.")),
    };
    if sheet.cells==object.workbook().sheets[0].cells {
        Ok(create("current","Embedded worksheet matches the file on disk."))
    }else{
        Ok(create("changed","Source cells changed on disk. Refresh will replace the unchanged embedded copy."))
    }
}
pub fn refresh(doc:&mut Document,expected_revision:u64,id:&str)->Result<(),String>{
    if doc.revision!=expected_revision{return Err("Writer revision conflict".into());}
    let mut next=doc.clone();
    let obj=next.embedded_objects.iter_mut().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    let origin=obj.file_link().ok_or("Not a file-linked sheet")?.clone();
    check_local(obj,&origin)?;
    let (_,source)=load_source(&origin.source_path)?;
    let tab=get_original(&origin,&source)?.clone();
    if obj.workbook().sheets[0].cells==tab.cells {return Ok(());}
    let workbook=obj.workbook_mut();
    workbook.sheets[0]=tab;
    workbook.revision=workbook.revision.checked_add(1).ok_or("Embedded revision overflow")?;
    let new_revision=workbook.revision;
    let link=obj.file_link_mut().as_mut().ok_or("File link missing")?;
    link.clean_embedded_revision=new_revision;
    link.source_revision=source.revision;
    next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
    next.validate()?;
    *doc=next;
    Ok(())
}
/// Reconnect requires a *new file choice* from the desktop file picker.
/// Reject local edits even when the new file has the same name.
pub fn reconnect(doc:&mut Document,expected_revision:u64,id:&str,path:&str,sheet_index:usize)->Result<(),String>{
    if doc.revision!=expected_revision{return Err("Writer revision conflict".into());}
    let (canonical,source)=load_source(path)?;
    let tab=source.sheets.get(sheet_index).ok_or("Source sheet index out of range")?.clone();
    let mut next=doc.clone();
    let object=next.embedded_objects.iter_mut().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    let original=object.file_link().ok_or("Not a file-linked sheet")?.clone();
    check_local(object,&original)?;
    let workbook=object.workbook_mut();
    workbook.sheets[0]=tab.clone();
    workbook.revision=workbook.revision.checked_add(1).ok_or("Embedded revision overflow")?;
    let clean=workbook.revision;
    *object.file_link_mut()=Some(FileSheetLink{
        source_path:canonical,source_title:source.title,source_sheet:tab.name,
        source_revision:source.revision,clean_embedded_revision:clean,
    });
    next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
    next.validate()?;
    *doc=next;
    Ok(())
}
pub fn detach(doc:&mut Document,expected_revision:u64,id:&str)->Result<(),String>{
    if doc.revision!=expected_revision{return Err("Writer revision conflict".into());}
    let mut next=doc.clone();
    let obj=next.embedded_objects.iter_mut().find(|o|o.id()==id).ok_or("Embedded object not found")?;
    if obj.file_link().is_none(){return Err("Not a file link".into());}
    *obj.file_link_mut()=None;
    next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
    next.validate()?;
    *doc=next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize,Ordering};
    static TEST_SEQ:AtomicUsize=AtomicUsize::new(1);
    fn path()->PathBuf{std::env::temp_dir().join(format!("koffice-fusion-file-{}-{}.ksheet",std::process::id(),TEST_SEQ.fetch_add(1,Ordering::Relaxed)))}
    fn fixture()->(Document,PathBuf,String){
        let mut book=SheetBook::blank();
        book.put(0,0,"A1".into(),"10".into()).unwrap();
        book.put(1,0,"B1".into(),"=A1*2".into()).unwrap();
        let dest=path();suite::save_json(&dest,&book).unwrap();
        let mut doc=Document::blank();
        attach(&mut doc,0,dest.to_str().unwrap(),0,"Finance".into()).unwrap();
        let id=doc.embedded_objects[0].id().to_string();
        (doc,dest,id)
    }
    #[test]fn persists_link_after_restart_and_refreshes_file_edits(){
        let (doc,path,id)=fixture();
        let serialized=serde_json::to_string(&doc).unwrap();
        let mut reopened:Document=serde_json::from_str(&serialized).unwrap();
        assert_eq!(status(&reopened,&id).unwrap().state,"current");
        let (_,mut source)=load_source(path.to_str().unwrap()).unwrap();
        source.put(source.revision,0,"A1".into(),"20".into()).unwrap();
        suite::save_json(&path,&source).unwrap();
        assert_eq!(status(&reopened,&id).unwrap().state,"changed");
        refresh(&mut reopened,1,&id).unwrap();
        assert_eq!(reopened.embedded_objects[0].workbook().sheets[0].cells["A1"],"20");
        assert_eq!(reopened.embedded_objects[0].workbook().sheets[0].cells["B1"],"=A1*2");
        assert_eq!(status(&reopened,&id).unwrap().state,"current");
        std::fs::remove_file(path).unwrap();
    }
    #[test]fn local_changes_block_file_refresh_even_if_source_changed(){
        let (mut doc,path,id)=fixture();
        let mut cells=BTreeMap::new();cells.insert("A1".into(),"99".into());
        doc.embedded_put(1,&id,cells).unwrap();
        let (_,mut source)=load_source(path.to_str().unwrap()).unwrap();
        source.put(source.revision,0,"A1".into(),"44".into()).unwrap();
        suite::save_json(&path,&source).unwrap();
        assert_eq!(status(&doc,&id).unwrap().state,"conflict");
        let before=doc.clone();
        assert!(refresh(&mut doc,2,&id).is_err());
        assert_eq!(doc,before);
        detach(&mut doc,2,&id).unwrap();
        assert!(doc.embedded_objects[0].file_link().is_none());
        assert_eq!(doc.embedded_objects[0].workbook().sheets[0].cells["A1"],"99");
        std::fs::remove_file(path).unwrap();
    }
    #[test]fn missing_file_and_wrong_identity_never_overwrite(){
        let (mut doc,path,id)=fixture();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(status(&doc,&id).unwrap().state,"disconnected");
        assert!(refresh(&mut doc,1,&id).is_err());
        let mut replacement=SheetBook::blank();
        replacement.title="Different identity".into();
        suite::save_json(&path,&replacement).unwrap();
        assert!(refresh(&mut doc,1,&id).is_err());
        let alternate=self::path();
        let mut other=SheetBook::blank();
        other.put(0,0,"A1".into(),"777".into()).unwrap();
        suite::save_json(&alternate,&other).unwrap();
        reconnect(&mut doc,1,&id,alternate.to_str().unwrap(),0).unwrap();
        assert_eq!(doc.embedded_objects[0].workbook().sheets[0].cells["A1"],"777");
        std::fs::remove_file(path).unwrap();std::fs::remove_file(alternate).unwrap();
    }
    #[test]fn invalid_source_fails_without_mutation(){
        let mut doc=Document::blank();
        let before=doc.clone();
        assert!(attach(&mut doc,0,"/not-a-file.txt",0,"Bad".into()).is_err());
        assert_eq!(before,doc);
    }
}
