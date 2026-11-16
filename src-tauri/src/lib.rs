mod engine;
mod suite;
mod sheet_tools;
mod base;
mod automation;
mod fusion;
mod fusion_transfer;
mod fusion_links;
mod fusion_files;
mod fusion_manager;
mod base_slide_links;
mod desktop;
use engine::{Document, Edit};
use suite::{Deck, SheetBook, Slide};
use std::{path::Path, sync::{Mutex,atomic::{AtomicU64,Ordering}},time::{SystemTime,UNIX_EPOCH}};
use base::{Database, Field, QueryResult, TableInfo, ImportResult, IndexInfo};
use tauri::State;

struct Workspace { writer: Mutex<Document>, sheets: Mutex<SheetBook>, slides: Mutex<Deck>, base: Mutex<Database>, sheets_namespace: String, sheets_generation: AtomicU64, fusion_preview: Mutex<Option<fusion_manager::BatchPreview>>, fusion_preview_sequence: AtomicU64 }
fn session_token(s:&Workspace)->String{
    format!("{}-{}",s.sheets_namespace,s.sheets_generation.load(Ordering::SeqCst))
}
fn lock<T>(mutex:&Mutex<T>)->Result<std::sync::MutexGuard<'_,T>,String>{mutex.lock().map_err(|_|"Workspace lock poisoned".into())}

#[tauri::command] fn snapshot(s:State<'_,Workspace>)->Result<Document,String>{Ok(lock(&s.writer)?.clone())}
#[tauri::command] fn new_document(s:State<'_,Workspace>)->Result<Document,String>{let mut d=lock(&s.writer)?;*d=Document::blank();Ok(d.clone())}
#[tauri::command] fn edit_document(s:State<'_,Workspace>,expected_revision:u64,edit:Edit)->Result<Document,String>{let mut d=lock(&s.writer)?;d.apply(expected_revision,edit)?;Ok(d.clone())}
#[tauri::command] fn writer_set_page(s:State<'_,Workspace>,expected_revision:u64,page:engine::PageSettings)->Result<Document,String>{
    let mut d=lock(&s.writer)?;
    d.set_page(expected_revision,page)?;
    Ok(d.clone())
}
#[tauri::command] fn rename_document(s:State<'_,Workspace>,expected_revision:u64,title:String)->Result<Document,String>{let mut d=lock(&s.writer)?;if d.revision!=expected_revision{return Err("Revision conflict".into());}let mut next=d.clone();next.title=title;next.revision=next.revision.checked_add(1).ok_or("Revision overflow")?;next.validate()?;*d=next;Ok(d.clone())}
#[tauri::command] fn open_document(s:State<'_,Workspace>,path:String)->Result<Document,String>{let d=engine::open(Path::new(&path))?;*lock(&s.writer)?=d.clone();Ok(d)}
#[tauri::command] fn save_document(s:State<'_,Workspace>,path:String)->Result<(),String>{engine::save(Path::new(&path),&*lock(&s.writer)?)}

/// Insert a self-contained sheet copy into Writer; the source workbook remains unchanged.
/// Lock order Writer -> Sheets matches the automation transaction ordering.
#[tauri::command]
fn writer_embed_sheet(s:State<'_,Workspace>,expected_revision:u64,label:String,source_sheet:Option<usize>)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    let source=match source_sheet {
        Some(index)=>{let source=lock(&s.sheets)?;let mut selected=SheetBook::blank();selected.title=source.title.clone();selected.sheets=vec![source.sheets.get(index).ok_or("Sheet not found")?.clone()];selected},
        None=>SheetBook::blank(),
    };
    writer.embed_sheet(expected_revision,label,&source,0)?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_embedded_put(s:State<'_,Workspace>,expected_revision:u64,object_id:String,updates:std::collections::BTreeMap<String,String>)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    writer.embedded_put(expected_revision,&object_id,updates)?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_remove_embedded(s:State<'_,Workspace>,expected_revision:u64,object_id:String)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    writer.remove_embedded(expected_revision,&object_id)?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_embedded_value(s:State<'_,Workspace>,object_id:String,address:String)->Result<String,String>{
    let writer=lock(&s.writer)?;
    let object=writer.embedded_objects.iter().find(|o|o.id()==object_id).ok_or("Embedded object not found")?;
    object.workbook().value(0,&address)
}

/// Lossless copy of an embedded Writer worksheet into the already-open Sheets workbook.
/// All current sheets survive. Lock ordering stays writer -> sheets.
#[tauri::command]
fn writer_embedded_to_sheets(
    s:State<'_,Workspace>, object_id:String, expected_revision:u64,
)->Result<SheetBook,String> {
    let writer=lock(&s.writer)?;
    let mut sheets=lock(&s.sheets)?;
    fusion_transfer::append_embedded_to_workbook(&writer,&object_id,&mut sheets,expected_revision)?;
    Ok(sheets.clone())
}

/// Appends a static first-page Base table snapshot to Slides; source DB and
/// existing presentation slides are unchanged. No live link is claimed.
#[tauri::command]
fn base_snapshot_to_slides(
    s:State<'_,Workspace>,table:String,expected_revision:u64,
)->Result<Deck,String> {
    let rows=lock(&s.base)?.browse_page(&table,None,0)?;
    let mut slides=lock(&s.slides)?;
    fusion_transfer::append_base_snapshot(&mut slides,expected_revision,&table,&rows)?;
    Ok(slides.clone())
}

/// A live link only points at the workbook currently opened in this process.
/// The session generation prevents accidental reconnection after new/open/restart.
#[tauri::command]
fn writer_embed_linked_sheet(s:State<'_,Workspace>,expected_revision:u64,label:String,source_sheet:usize)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    let source=lock(&s.sheets)?;
    fusion_links::create_link(&mut writer,expected_revision,label,&source,source_sheet,&session_token(&s))?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_link_status(s:State<'_,Workspace>,object_id:String)->Result<fusion_links::LinkStatus,String>{
    let writer=lock(&s.writer)?;
    let source=lock(&s.sheets)?;
    fusion_links::status(&writer,&object_id,&source,&session_token(&s))
}
#[tauri::command]
fn writer_refresh_link(s:State<'_,Workspace>,expected_revision:u64,object_id:String)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    let source=lock(&s.sheets)?;
    fusion_links::refresh(&mut writer,expected_revision,&object_id,&source,&session_token(&s))?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_reconnect_link(s:State<'_,Workspace>,expected_revision:u64,object_id:String,source_sheet:usize)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    let source=lock(&s.sheets)?;
    fusion_links::reconnect(&mut writer,expected_revision,&object_id,&source,source_sheet,&session_token(&s))?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_detach_link(s:State<'_,Workspace>,expected_revision:u64,object_id:String)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    fusion_links::detach(&mut writer,expected_revision,&object_id)?;
    Ok(writer.clone())
}

/// The UI's file picker provides a .ksheet path. Never search the filesystem automatically.
#[tauri::command]
fn fusion_file_sheets(path:String)->Result<Vec<String>,String>{fusion_files::list_sheets(&path)}
#[tauri::command]
fn writer_attach_file_sheet(s:State<'_,Workspace>,expected_revision:u64,label:String,path:String,source_sheet:usize)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    fusion_files::attach(&mut writer,expected_revision,&path,source_sheet,label)?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_file_link_status(s:State<'_,Workspace>,object_id:String)->Result<fusion_files::FileLinkStatus,String>{
    let writer=lock(&s.writer)?;
    fusion_files::status(&writer,&object_id)
}
#[tauri::command]
fn writer_refresh_file_link(s:State<'_,Workspace>,expected_revision:u64,object_id:String)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    fusion_files::refresh(&mut writer,expected_revision,&object_id)?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_reconnect_file_link(s:State<'_,Workspace>,expected_revision:u64,object_id:String,path:String,source_sheet:usize)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    fusion_files::reconnect(&mut writer,expected_revision,&object_id,&path,source_sheet)?;
    Ok(writer.clone())
}
#[tauri::command]
fn writer_detach_file_link(s:State<'_,Workspace>,expected_revision:u64,object_id:String)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    fusion_files::detach(&mut writer,expected_revision,&object_id)?;
    Ok(writer.clone())
}

/// Read-only inventory of all Fusion objects; does not follow any file paths.
#[tauri::command]
fn fusion_graph(s:State<'_,Workspace>)->Result<Vec<fusion_manager::LinkNode>,String>{
    let writer=lock(&s.writer)?;
    let sheets=lock(&s.sheets)?;
    Ok(fusion_manager::graph(&writer,&sheets,&session_token(&s)))
}
/// Reads only explicitly user-approved file-backed sources. The returned
/// token binds Apply to a preview of exact source sheets and Writer revision.
#[tauri::command]
fn fusion_preview_refresh(s:State<'_,Workspace>,object_ids:Vec<String>)->Result<fusion_manager::PreviewReport,String>{
    let writer=lock(&s.writer)?;
    let token=s.fusion_preview_sequence.fetch_add(1,Ordering::SeqCst);
    // Drop the previous preview first; a failed preview cannot leave a valid stale token.
    let mut slot=lock(&s.fusion_preview)?;
    *slot=None;
    let preview=fusion_manager::preview(&writer,&object_ids,token)?;
    let report=preview.report();
    *slot=Some(preview);
    Ok(report)
}
/// Atomic batch commit. Rechecks all file bytes/revisions, then swaps the
/// Writer document once. A stale or failed preview cannot be reused.
#[tauri::command]
fn fusion_apply_refresh(s:State<'_,Workspace>,token:u64,expected_revision:u64)->Result<Document,String>{
    let mut writer=lock(&s.writer)?;
    let mut slot=lock(&s.fusion_preview)?;
    let batch=slot.take().ok_or("No pending Fusion preview; scan again")?;
    if writer.revision!=expected_revision || batch.report().expected_revision!=expected_revision {
        return Err("Writer revision changed; scan again".into());
    }
    fusion_manager::apply(&mut writer,&batch,token)?;
    Ok(writer.clone())
}

#[tauri::command] fn workbook_snapshot(s:State<'_,Workspace>)->Result<SheetBook,String>{Ok(lock(&s.sheets)?.clone())}
#[tauri::command] fn workbook_new(s:State<'_,Workspace>)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;*d=SheetBook::blank();s.sheets_generation.fetch_add(1,Ordering::SeqCst);Ok(d.clone())}
#[tauri::command] fn workbook_put(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,address:String,value:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.put(expected_revision,sheet,address,value)?;Ok(d.clone())}
#[tauri::command] fn workbook_add_sheet(s:State<'_,Workspace>,expected_revision:u64,name:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.add_sheet(expected_revision,name)?;Ok(d.clone())}
#[tauri::command] fn workbook_put_batch(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,updates:std::collections::BTreeMap<String,String>)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.put_batch(expected_revision,sheet,updates)?;Ok(d.clone())}
#[tauri::command] fn workbook_rename_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize,name:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.rename_sheet(expected_revision,index,name)?;Ok(d.clone())}
#[tauri::command] fn workbook_delete_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.delete_sheet(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn workbook_value(s:State<'_,Workspace>,sheet:usize,address:String)->Result<String,String>{lock(&s.sheets)?.value(sheet,&address)}
#[tauri::command] fn workbook_find(s:State<'_,Workspace>,needle:String)->Result<Vec<suite::CellMatch>,String>{lock(&s.sheets)?.find_cells(&needle,100)}
#[tauri::command] fn workbook_sort_range(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,range:String,key_column:String,ascending:bool,header:bool)->Result<SheetBook,String>{
 let mut book=lock(&s.sheets)?;
 sheet_tools::sort_range(&mut book,expected_revision,sheet,&range,&key_column,ascending,header)?;
 Ok(book.clone())
}
#[tauri::command] fn workbook_fill_series(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,range:String,start:f64,step:f64)->Result<SheetBook,String>{
 let mut book=lock(&s.sheets)?;
 sheet_tools::fill_series(&mut book,expected_revision,sheet,&range,start,step)?;
 Ok(book.clone())
}
#[tauri::command] fn workbook_clear_range(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,range:String)->Result<SheetBook,String>{
 let mut book=lock(&s.sheets)?;
 sheet_tools::clear_range(&mut book,expected_revision,sheet,&range)?;
 Ok(book.clone())
}
#[tauri::command] fn workbook_replace_text(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,range:String,needle:String,replacement:String)->Result<(SheetBook,usize),String>{
 let mut book=lock(&s.sheets)?;
 let count=sheet_tools::replace_text(&mut book,expected_revision,sheet,&range,&needle,&replacement)?;
 Ok((book.clone(),count))
}
#[tauri::command] fn workbook_style_cells(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,updates:std::collections::BTreeMap<String,Option<suite::CellStyle>>)->Result<SheetBook,String>{
 let mut book=lock(&s.sheets)?;
 book.style_cells(expected_revision,sheet,updates)?;
 Ok(book.clone())
}
#[tauri::command] fn workbook_set_freeze(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,freeze:Option<suite::SheetFreeze>)->Result<SheetBook,String>{
 let mut book=lock(&s.sheets)?;
 book.set_freeze(expected_revision,sheet,freeze)?;
 Ok(book.clone())
}
#[tauri::command] fn workbook_restore_snapshot(s:State<'_,Workspace>,expected_revision:u64,snapshot:SheetBook)->Result<SheetBook,String>{
 let mut book=lock(&s.sheets)?;
 book.restore_snapshot(expected_revision,snapshot)?;
 Ok(book.clone())
}
#[tauri::command] fn workbook_statistics(s:State<'_,Workspace>,sheet:usize)->Result<suite::SheetStatistics,String>{lock(&s.sheets)?.statistics(sheet)}
#[tauri::command] fn workbook_open(s:State<'_,Workspace>,path:String)->Result<SheetBook,String>{let d:SheetBook=suite::load_json(Path::new(&path))?;d.validate()?;*lock(&s.sheets)?=d.clone();s.sheets_generation.fetch_add(1,Ordering::SeqCst);Ok(d)}
#[tauri::command] fn workbook_save(s:State<'_,Workspace>,path:String)->Result<(),String>{let d=lock(&s.sheets)?;d.validate()?;suite::save_json(Path::new(&path),&*d)}

/// Insert the complete cell/formula model, never just a screenshot.
#[tauri::command]
fn slides_embed_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize,label:String,source_sheet:Option<usize>)->Result<Deck,String>{
    let source=match source_sheet {
        Some(i)=>{let sheets=lock(&s.sheets)?;let tab=sheets.sheets.get(i).ok_or("Source sheet not found")?.clone();
            let mut selected=SheetBook::blank();selected.title=sheets.title.clone();selected.sheets=vec![tab];selected},
        None=>SheetBook::blank(),
    };
    let mut deck=lock(&s.slides)?;
    deck.embed_sheet(expected_revision,index,label,&source,0)?;
    Ok(deck.clone())
}
#[tauri::command]
fn slides_embedded_put(s:State<'_,Workspace>,expected_revision:u64,index:usize,object_id:String,updates:std::collections::BTreeMap<String,String>)->Result<Deck,String>{
    let mut deck=lock(&s.slides)?;
    deck.embedded_put(expected_revision,index,&object_id,updates)?;
    Ok(deck.clone())
}
#[tauri::command]
fn slides_embedded_frame(s:State<'_,Workspace>,expected_revision:u64,index:usize,object_id:String,frame:suite::SlideObjectFrame)->Result<Deck,String>{
    let mut deck=lock(&s.slides)?;
    deck.move_embedded(expected_revision,index,&object_id,frame)?;
    Ok(deck.clone())
}
#[tauri::command]
fn slides_embedded_remove(s:State<'_,Workspace>,expected_revision:u64,index:usize,object_id:String)->Result<Deck,String>{
    let mut deck=lock(&s.slides)?;
    deck.remove_embedded(expected_revision,index,&object_id)?;
    Ok(deck.clone())
}
#[tauri::command]
fn slides_embedded_value(s:State<'_,Workspace>,index:usize,object_id:String,address:String)->Result<String,String>{
    lock(&s.slides)?.embedded_value(index,&object_id,&address)
}
#[tauri::command]
fn slides_embedded_to_sheets(s:State<'_,Workspace>,index:usize,object_id:String,expected_revision:u64)->Result<SheetBook,String>{
    // Match the global Sheets -> Slides lock order used by automation.
    let mut sheets=lock(&s.sheets)?;
    let deck=lock(&s.slides)?;
    fusion_transfer::append_slide_embedded_to_workbook(&deck,index,&object_id,&mut sheets,expected_revision)?;
    Ok(sheets.clone())
}

#[tauri::command] fn deck_snapshot(s:State<'_,Workspace>)->Result<Deck,String>{Ok(lock(&s.slides)?.clone())}
#[tauri::command] fn deck_new(s:State<'_,Workspace>)->Result<Deck,String>{let mut d=lock(&s.slides)?;*d=Deck::blank();Ok(d.clone())}
#[tauri::command] fn deck_add(s:State<'_,Workspace>,expected_revision:u64)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.add(expected_revision)?;Ok(d.clone())}
#[tauri::command] fn deck_update(s:State<'_,Workspace>,expected_revision:u64,index:usize,slide:Slide)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.update(expected_revision,index,slide)?;Ok(d.clone())}
#[tauri::command] fn deck_delete(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.delete(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn deck_duplicate(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.duplicate(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn deck_move(s:State<'_,Workspace>,expected_revision:u64,from:usize,to:usize)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.move_slide(expected_revision,from,to)?;Ok(d.clone())}
#[tauri::command] fn deck_open(s:State<'_,Workspace>,path:String)->Result<Deck,String>{let d:Deck=suite::load_json(Path::new(&path))?;d.validate()?;*lock(&s.slides)?=d.clone();Ok(d)}
#[tauri::command] fn deck_save(s:State<'_,Workspace>,path:String)->Result<(),String>{let d=lock(&s.slides)?;d.validate()?;suite::save_json(Path::new(&path),&*d)}

/// Explicit opt-in: append a slide linked to the currently open Base file.
/// A complete static snapshot is stored in the .kslide for offline display.
#[tauri::command]
fn base_link_to_slides(s:State<'_,Workspace>,table:String,expected_revision:u64)->Result<Deck,String>{
    let path=lock(&s.base)?.path().ok_or("Open a database in Base first")?;
    let mut deck=lock(&s.slides)?;
    base_slide_links::create(&mut deck,expected_revision,&path,&table)?;
    Ok(deck.clone())
}
/// Only call after UI shows the stored source path and receives approval.
#[tauri::command]
fn slides_base_link_status(s:State<'_,Workspace>,index:usize)->Result<base_slide_links::BaseSlideLinkStatus,String>{
    let deck=lock(&s.slides)?;
    base_slide_links::inspect(&deck,index)
}
/// Only call after user approves the exact stored source path.
#[tauri::command]
fn slides_base_link_refresh(s:State<'_,Workspace>,index:usize,expected_revision:u64)->Result<Deck,String>{
    let mut deck=lock(&s.slides)?;
    base_slide_links::refresh(&mut deck,expected_revision,index)?;
    Ok(deck.clone())
}
#[tauri::command]
fn slides_base_link_reconnect(s:State<'_,Workspace>,index:usize,expected_revision:u64,path:String)->Result<Deck,String>{
    let mut deck=lock(&s.slides)?;
    base_slide_links::reconnect(&mut deck,expected_revision,index,&path)?;
    Ok(deck.clone())
}
#[tauri::command]
fn slides_base_link_detach(s:State<'_,Workspace>,index:usize,expected_revision:u64)->Result<Deck,String>{
    let mut deck=lock(&s.slides)?;
    base_slide_links::detach(&mut deck,expected_revision,index)?;
    Ok(deck.clone())
}

#[tauri::command] fn base_path(s:State<'_,Workspace>)->Result<Option<String>,String>{Ok(lock(&s.base)?.path())}
#[tauri::command] fn base_create(s:State<'_,Workspace>,path:String)->Result<(),String>{lock(&s.base)?.create(&path)}
#[tauri::command] fn base_open(s:State<'_,Workspace>,path:String)->Result<(),String>{lock(&s.base)?.open(&path)}
#[tauri::command] fn base_tables(s:State<'_,Workspace>)->Result<Vec<TableInfo>,String>{lock(&s.base)?.tables()}
#[tauri::command] fn base_create_table(s:State<'_,Workspace>,table:String,fields:Vec<Field>)->Result<(),String>{lock(&s.base)?.create_table(&table,fields)}
#[tauri::command] fn base_insert(s:State<'_,Workspace>,table:String,values:serde_json::Map<String,serde_json::Value>)->Result<i64,String>{lock(&s.base)?.insert(&table,values)}
#[tauri::command] fn base_update(s:State<'_,Workspace>,table:String,id:i64,values:serde_json::Map<String,serde_json::Value>)->Result<usize,String>{lock(&s.base)?.update(&table,id,values)}
#[tauri::command] fn base_delete(s:State<'_,Workspace>,table:String,id:i64)->Result<usize,String>{lock(&s.base)?.delete(&table,id)}
#[tauri::command] fn base_browse(s:State<'_,Workspace>,table:String,search:Option<String>)->Result<QueryResult,String>{lock(&s.base)?.browse(&table,search)}
#[tauri::command] fn base_select(s:State<'_,Workspace>,sql:String)->Result<QueryResult,String>{lock(&s.base)?.select(&sql)}

#[tauri::command] fn base_import_csv(s:State<'_,Workspace>,table:String,content:String)->Result<ImportResult,String>{lock(&s.base)?.import_csv(&table,&content)}
#[tauri::command] fn base_indexes(s:State<'_,Workspace>,table:String)->Result<Vec<IndexInfo>,String>{lock(&s.base)?.indexes(&table)}
#[tauri::command] fn base_create_index(s:State<'_,Workspace>,table:String,columns:Vec<String>,unique:bool)->Result<String,String>{lock(&s.base)?.create_index(&table,columns,unique)}
#[tauri::command] fn base_delete_index(s:State<'_,Workspace>,name:String)->Result<(),String>{lock(&s.base)?.delete_index(&name)}
#[tauri::command] fn base_browse_page(s:State<'_,Workspace>,table:String,search:Option<String>,offset:usize)->Result<QueryResult,String>{lock(&s.base)?.browse_page(&table,search,offset)}

#[tauri::command] fn base_integrity_check(s:State<'_,Workspace>)->Result<String,String>{lock(&s.base)?.integrity_check()}
#[tauri::command] fn base_backup(s:State<'_,Workspace>,destination:String)->Result<(),String>{lock(&s.base)?.backup(&destination)}
#[tauri::command] fn base_export_table_csv(s:State<'_,Workspace>,table:String)->Result<String,String>{lock(&s.base)?.export_table_csv(&table)}
#[tauri::command] fn base_group_report(s:State<'_,Workspace>,table:String,group_field:String,metric_field:Option<String>)->Result<QueryResult,String>{lock(&s.base)?.group_report(&table,&group_field,metric_field.as_deref())}

/// Snapshot a database table into Sheets using a single workbook revision.
#[tauri::command] fn base_to_workbook(s:State<'_,Workspace>,table:String)->Result<SheetBook,String>{
    let rows=lock(&s.base)?.browse_page(&table,None,0)?;
    if (rows.rows.len()+1)*rows.columns.len()>20_000{return Err("Table snapshot exceeds 20,000 cells".into());}
    let mut updates=std::collections::BTreeMap::new();
    for (col,name) in rows.columns.iter().enumerate(){updates.insert(cell_address(1,col+1),name.clone());}
    for (row,values) in rows.rows.iter().enumerate(){
        for (col,value) in values.iter().enumerate(){
            let text=match value{serde_json::Value::Null=>String::new(),serde_json::Value::String(s)=>s.clone(),v=>v.to_string()};
            if !text.is_empty(){updates.insert(cell_address(row+2,col+1),text);}
        }
    }
    let mut book=SheetBook::blank();
    book.sheets[0].name=table.chars().take(80).collect();
    book.put_batch(0,0,updates)?;
    *lock(&s.sheets)?=book.clone();s.sheets_generation.fetch_add(1,Ordering::SeqCst);
    Ok(book)
}
fn cell_address(row:usize,mut col:usize)->String{
    let mut label=String::new();while col>0{col-=1;label.insert(0,(b'A'+(col%26)as u8)as char);col/=26;}format!("{label}{row}")
}

/// Read-only snapshot of the currently opened Writer, Sheets and Slides workspaces.
#[tauri::command]
fn automation_snapshot(s:State<'_,Workspace>)->Result<automation::AutomationSnapshot,String>{
    Ok(automation::AutomationSnapshot{
        writer:lock(&s.writer)?.clone(),
        sheets:lock(&s.sheets)?.clone(),
        slides:lock(&s.slides)?.clone(),
    })
}

/// Serialize cross-editor updates by acquiring locks in a single fixed order.
#[tauri::command]
fn automation_execute(s:State<'_,Workspace>,batch:automation::AutomationBatch)->Result<automation::AutomationResult,String>{
    let mut writer=lock(&s.writer)?;
    let mut sheets=lock(&s.sheets)?;
    let mut slides=lock(&s.slides)?;
    automation::execute(&mut writer,&mut sheets,&mut slides,batch)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(){tauri::Builder::default().setup(|app|{use tauri::Manager;let mode=desktop::DesktopMode::from_args(std::env::args().skip(1));if let Some(window)=app.get_webview_window("main"){window.set_title(mode.window_title())?;}Ok(())}).plugin(tauri_plugin_dialog::init()).manage(Workspace{writer:Mutex::new(Document::blank()),sheets:Mutex::new(SheetBook::blank()),slides:Mutex::new(Deck::blank()),base:Mutex::new(Database::default()),sheets_namespace:format!("{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()),sheets_generation:AtomicU64::new(1),fusion_preview:Mutex::new(None),fusion_preview_sequence:AtomicU64::new(1)}).invoke_handler(tauri::generate_handler![desktop::desktop_mode,desktop::desktop_launch,snapshot,new_document,edit_document,writer_set_page,rename_document,open_document,save_document,writer_embed_sheet,writer_embedded_put,writer_remove_embedded,writer_embedded_value,writer_embedded_to_sheets,base_snapshot_to_slides,base_link_to_slides,slides_embed_sheet,slides_embedded_put,slides_embedded_frame,slides_embedded_remove,slides_embedded_value,slides_embedded_to_sheets,slides_base_link_status,slides_base_link_refresh,slides_base_link_reconnect,slides_base_link_detach,writer_embed_linked_sheet,writer_link_status,writer_refresh_link,writer_reconnect_link,writer_detach_link,fusion_file_sheets,writer_attach_file_sheet,writer_file_link_status,writer_refresh_file_link,writer_reconnect_file_link,writer_detach_file_link,fusion_graph,fusion_preview_refresh,fusion_apply_refresh,workbook_snapshot,workbook_new,workbook_put,workbook_add_sheet,workbook_put_batch,workbook_rename_sheet,workbook_delete_sheet,workbook_value,workbook_find,workbook_statistics,workbook_sort_range,workbook_fill_series,workbook_clear_range,workbook_replace_text,workbook_style_cells,workbook_set_freeze,workbook_restore_snapshot,workbook_open,workbook_save,deck_snapshot,deck_new,deck_add,deck_update,deck_delete,deck_duplicate,deck_move,deck_open,deck_save,base_path,base_create,base_open,base_tables,base_create_table,base_insert,base_update,base_delete,base_browse,base_select,base_import_csv,base_indexes,base_create_index,base_delete_index,base_browse_page,base_to_workbook,base_integrity_check,base_backup,base_export_table_csv,base_group_report,automation_snapshot,automation_execute]).run(tauri::generate_context!()).expect("Failed to start Koffice")}