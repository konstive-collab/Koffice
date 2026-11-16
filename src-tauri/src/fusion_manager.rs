//! Koffice Fusion v0.14: inspectable graph and two-phase, atomic file-link refresh.
//! Listing a graph never follows external paths. Preview is an explicit user action;
//! commit revalidates the exact previewed cells before modifying any Writer object.
use crate::{engine::Document, suite::{Sheet, SheetBook}, fusion_files, fusion_links};
use serde::Serialize;
use std::collections::{BTreeMap,BTreeSet};

const MAX_REFRESH_OBJECTS: usize=12;
const MAX_DIFF_EXAMPLES: usize=8;

#[derive(Clone,Debug,Serialize)]
pub struct LinkNode {
    pub id:String, pub label:String, pub mode:String, pub source:String,
    pub worksheet:String, pub embedded_revision:u64, pub status:String,
}

pub fn graph(doc:&Document,workbook:&SheetBook,session_token:&str)->Vec<LinkNode> {
    doc.embedded_objects.iter().map(|obj|{
        let mode=if obj.file_link().is_some(){"file"}else if obj.link().is_some(){"session"}else{"embedded"};
        let (source,worksheet,state)=if let Some(l)=obj.file_link(){
            (l.source_path.clone(),l.source_sheet.clone(),
             if obj.workbook().revision!=l.clean_embedded_revision {"local_edits"}else{"unchecked"})
        }else if let Some(l)=obj.link(){
            let status=fusion_links::status(doc,obj.id(),workbook,session_token).ok();
            (l.source_title.clone(),l.source_sheet.clone(),
             match status.as_ref().map(|x|x.state.as_str()) {
                 Some("current")=>"current",Some("changed")=>"changed",
                 Some("conflict")=>"local_edits",_=>"disconnected"
             })
        }else{
            (String::new(),String::new(),"independent")
        };
        let label=match obj{crate::fusion::EmbeddedObject::Sheet{label,..}=>label.clone()};
        LinkNode{id:obj.id().into(),label,mode:mode.into(),source,worksheet,embedded_revision:obj.workbook().revision,status:state.into()}
    }).collect()
}

#[derive(Clone,Debug,Serialize)]
pub struct DiffExample {pub address:String,pub embedded:String,pub source:String}
#[derive(Clone,Debug,Serialize)]
pub struct PreviewRow {
    pub id:String,pub label:String,pub source_path:String,pub worksheet:String,
    pub state:String,pub changed_cells:usize,pub examples:Vec<DiffExample>,pub detail:String,
}
#[derive(Clone,Debug,Serialize)]
pub struct PreviewReport {pub token:u64,pub expected_revision:u64,pub rows:Vec<PreviewRow>}
#[derive(Clone,Debug)]
struct ApprovedSource {id:String,path:String,revision:u64,title:String,sheet:Sheet,state:String}
#[derive(Clone,Debug)]
pub struct BatchPreview {report:PreviewReport,sources:Vec<ApprovedSource>}
impl BatchPreview {
    pub fn report(&self)->PreviewReport{self.report.clone()}
}
fn source_matching(link:&crate::fusion::FileSheetLink,source:&SheetBook)->Result<Sheet,String>{
    if source.title!=link.source_title {return Err("Source workbook title changed".into());}
    source.sheets.iter().find(|tab|tab.name==link.source_sheet)
        .cloned().ok_or_else(||"Linked worksheet renamed or removed".into())
}
fn diff(old:&BTreeMap<String,String>,new:&BTreeMap<String,String>)->(usize,Vec<DiffExample>) {
    let keys:BTreeSet<String>=old.keys().chain(new.keys()).cloned().collect();
    let mut count=0;let mut examples=Vec::new();
    for key in keys {
        let a=old.get(&key).map(String::as_str).unwrap_or("");
        let b=new.get(&key).map(String::as_str).unwrap_or("");
        if a==b {continue;}
        count+=1;
        if examples.len()<MAX_DIFF_EXAMPLES {
            examples.push(DiffExample{address:key.clone(),embedded:a.chars().take(100).collect(),source:b.chars().take(100).collect()});
        }
    }
    (count,examples)
}
/// Preview selected file-backed links. External reads must be preceded by UI
/// confirmation showing every stored path. Conflicting and missing sources are
/// reported as blocked; no document or source is modified.
pub fn preview(doc:&Document,ids:&[String],token:u64)->Result<BatchPreview,String>{
    doc.validate()?;
    if ids.is_empty()||ids.len()>MAX_REFRESH_OBJECTS {return Err("Select 1 to 12 linked tables".into());}
    let mut seen=BTreeSet::new();
    let mut rows=Vec::new();
    let mut sources=Vec::new();
    for id in ids{
        if !seen.insert(id.as_str()) {return Err("Duplicate selected table".into());}
        let obj=doc.embedded_objects.iter().find(|obj|obj.id()==id).ok_or("Embedded table not found")?;
        let link=obj.file_link().ok_or("Only file-backed links can be refreshed in this batch")?;
        let label=match obj{crate::fusion::EmbeddedObject::Sheet{label,..}=>label.clone()};
        let mut row=PreviewRow{id:id.clone(),label,source_path:link.source_path.clone(),worksheet:link.source_sheet.clone(),
            state:String::new(),changed_cells:0,examples:vec![],detail:String::new()};
        if obj.workbook().revision!=link.clean_embedded_revision {
            row.state="conflict".into();
            row.detail="Local embedded edits are protected. Detach the table to preserve them.".into();
        }else{
            match fusion_files::load_source(&link.source_path) {
                Err(_)=>{
                    row.state="disconnected".into();
                    row.detail="Source file cannot be read. Relink through the file picker.".into();
                },
                Ok((canonical,book))=>match source_matching(link,&book) {
                    Err(_)=>{
                        row.state="disconnected".into();
                        row.detail="Source identity changed. Relink explicitly.".into();
                    },
                    Ok(tab)=>{
                        let (count,examples)=diff(&obj.workbook().sheets[0].cells,&tab.cells);
                        row.changed_cells=count;row.examples=examples;
                        row.state=if count==0{"current"}else{"changed"}.into();
                        row.detail=if count==0{"No cell differences."}else{"Review the differences before applying."}.into();
                        sources.push(ApprovedSource{id:id.clone(),path:canonical,revision:book.revision,title:book.title,sheet:tab,state:row.state.clone()});
                    }
                }
            }
        }
        rows.push(row);
    }
    Ok(BatchPreview{report:PreviewReport{token,expected_revision:doc.revision,rows},sources})
}

/// All-or-nothing: refuse conflicts, changed source bytes, changed Writer
/// revision, unexpected preview token, or missing sources. No writes to source files.
pub fn apply(doc:&mut Document,approved:&BatchPreview,token:u64)->Result<usize,String>{
    if token!=approved.report.token {return Err("Preview token no longer matches".into());}
    if doc.revision!=approved.report.expected_revision {return Err("Writer changed after preview; run preview again".into());}
    if approved.report.rows.iter().any(|row|row.state!="changed"&&row.state!="current") {
        return Err("Selected links include conflicts or disconnected files. Exclude or repair them first.".into());
    }
    if approved.sources.len()!=approved.report.rows.len(){return Err("Incomplete source preview".into());}
    let mut next=doc.clone();
    let mut changes=0;
    for source in &approved.sources {
        let object=next.embedded_objects.iter_mut().find(|obj|obj.id()==source.id).ok_or("Embedded object was removed")?;
        let link=object.file_link().ok_or("Link was removed")?.clone();
        if object.workbook().revision!=link.clean_embedded_revision {
            return Err("Local edits occurred; refresh refused".into());
        }
        if link.source_path!=source.path || link.source_title!=source.title || link.source_sheet!=source.sheet.name {
            return Err("Preview link identity changed".into());
        }
        // Explicit re-read at Apply: reject changes between Preview and Apply.
        let (canonical,current)=fusion_files::load_source(&source.path)?;
        let tab=source_matching(&link,&current)?;
        if canonical!=source.path || current.revision!=source.revision || tab!=source.sheet {
            return Err("Source file changed after preview; preview again".into());
        }
        if source.state=="current" {continue;}
        let workbook=object.workbook_mut();
        workbook.sheets[0]=tab;
        workbook.revision=workbook.revision.checked_add(1).ok_or("Embedded revision overflow")?;
        let new_revision=workbook.revision;
        let target=object.file_link_mut().as_mut().ok_or("File link missing")?;
        target.source_revision=current.revision;
        target.clean_embedded_revision=new_revision;
        changes+=1;
    }
    if changes!=0 {
        next.revision=next.revision.checked_add(1).ok_or("Writer revision overflow")?;
        next.validate()?;
        *doc=next;
    }
    Ok(changes)
}

#[cfg(test)]
mod tests{
    use super::*;
    use std::sync::atomic::{AtomicU64,Ordering};
    static SERIAL:AtomicU64=AtomicU64::new(1);
    struct TestSource(std::path::PathBuf);
    impl Drop for TestSource{fn drop(&mut self){let _=std::fs::remove_file(&self.0);}}
    fn fixture()->(Document,Vec<TestSource>,Vec<String>){
        let mut d=Document::blank();let mut files=Vec::new();let mut ids=Vec::new();
        for i in 0..2 {
            let path=std::env::temp_dir().join(format!("koffice-batch-{}-{}-{}.ksheet",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed),i));
            let mut b=SheetBook::blank();
            b.put(0,0,"A1".into(),format!("{i}")).unwrap();
            suite_save(&path,&b);
            let revision=d.revision;
            fusion_files::attach(&mut d,revision,path.to_str().unwrap(),0,format!("Table {i}")).unwrap();
            ids.push(d.embedded_objects.last().unwrap().id().to_string());
            files.push(TestSource(path));
        }
        (d,files,ids)
    }
    fn suite_save(path:&std::path::Path,book:&SheetBook){crate::suite::save_json(path,book).unwrap();}
    #[test]fn graph_never_reads_disk(){
        let (doc,files,_)=fixture();
        std::fs::remove_file(&files[0].0).unwrap();
        let graph=graph(&doc,&SheetBook::blank(),"dummy");
        assert_eq!(graph.len(),2);
        assert_eq!(graph[0].status,"unchecked");
    }
    #[test]fn preview_then_apply_is_atomic_for_two_sources(){
        let (mut doc,files,ids)=fixture();
        for f in &files {
            let (_,mut book)=fusion_files::load_source(f.0.to_str().unwrap()).unwrap();
            book.put(book.revision,0,"B1".into(),"=A1+10".into()).unwrap();
            suite_save(&f.0,&book);
        }
        let before=doc.revision;
        let preview=preview(&doc,&ids,21).unwrap();
        assert_eq!(preview.report.rows.iter().filter(|x|x.state=="changed").count(),2);
        assert_eq!(preview.report.rows[0].changed_cells,1);
        assert!(apply(&mut doc,&preview,22).is_err());
        assert_eq!(doc.revision,before);
        assert_eq!(apply(&mut doc,&preview,21).unwrap(),2);
        assert_eq!(doc.revision,before+1);
        assert!(doc.embedded_objects.iter().all(|o|o.workbook().sheets[0].cells.contains_key("B1")));
    }
    #[test]fn changed_source_between_preview_and_apply_rolls_back_all(){
        let (mut doc,files,ids)=fixture();
        for f in &files {
            let (_,mut b)=fusion_files::load_source(f.0.to_str().unwrap()).unwrap();
            b.put(b.revision,0,"B1".into(),"30".into()).unwrap();
            suite_save(&f.0,&b);
        }
        let pending=preview(&doc,&ids,5).unwrap();
        let (_,mut b)=fusion_files::load_source(files[1].0.to_str().unwrap()).unwrap();
        b.put(b.revision,0,"B2".into(),"unexpected".into()).unwrap();suite_save(&files[1].0,&b);
        let before=doc.clone();
        assert!(apply(&mut doc,&pending,5).is_err());
        assert_eq!(doc,before);
    }
    #[test]fn edited_embedded_sheet_blocks_batch_and_stays_intact(){
        let (mut doc,_files,ids)=fixture();
        let mut edits=BTreeMap::new();edits.insert("A1".into(),"local".into());
        doc.embedded_put(doc.revision,&ids[1],edits).unwrap();
        let report=preview(&doc,&ids,9).unwrap();
        assert!(report.report.rows.iter().any(|r|r.state=="conflict"));
        let before=doc.clone();
        assert!(apply(&mut doc,&report,9).is_err());
        assert_eq!(doc,before);
    }
    #[test]fn unlinked_legacy_and_duplicate_selection_rejected(){
        let (doc,_files,ids)=fixture();
        assert!(preview(&doc,&[ids[0].clone(),ids[0].clone()],1).is_err());
        let mut old=Document::blank();
        old.embed_sheet(0,"Old".into(),&SheetBook::blank(),0).unwrap();
        assert_eq!(graph(&old,&SheetBook::blank(),"unknown")[0].mode,"embedded");
        assert!(preview(&old,&[old.embedded_objects[0].id().into()],2).is_err());
    }
}
