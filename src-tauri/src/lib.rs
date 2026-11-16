mod engine;
mod suite;
mod base;
mod automation;
mod fusion;
use engine::{Document, Edit};
use suite::{Deck, SheetBook, Slide};
use std::{path::Path, sync::Mutex};
use base::{Database, Field, QueryResult, TableInfo, ImportResult, IndexInfo};
use tauri::State;

struct Workspace { writer: Mutex<Document>, sheets: Mutex<SheetBook>, slides: Mutex<Deck>, base: Mutex<Database> }
fn lock<T>(mutex:&Mutex<T>)->Result<std::sync::MutexGuard<'_,T>,String>{mutex.lock().map_err(|_|"Workspace lock poisoned".into())}

#[tauri::command] fn snapshot(s:State<'_,Workspace>)->Result<Document,String>{Ok(lock(&s.writer)?.clone())}
#[tauri::command] fn new_document(s:State<'_,Workspace>)->Result<Document,String>{let mut d=lock(&s.writer)?;*d=Document::blank();Ok(d.clone())}
#[tauri::command] fn edit_document(s:State<'_,Workspace>,expected_revision:u64,edit:Edit)->Result<Document,String>{let mut d=lock(&s.writer)?;d.apply(expected_revision,edit)?;Ok(d.clone())}
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

#[tauri::command] fn workbook_snapshot(s:State<'_,Workspace>)->Result<SheetBook,String>{Ok(lock(&s.sheets)?.clone())}
#[tauri::command] fn workbook_new(s:State<'_,Workspace>)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;*d=SheetBook::blank();Ok(d.clone())}
#[tauri::command] fn workbook_put(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,address:String,value:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.put(expected_revision,sheet,address,value)?;Ok(d.clone())}
#[tauri::command] fn workbook_add_sheet(s:State<'_,Workspace>,expected_revision:u64,name:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.add_sheet(expected_revision,name)?;Ok(d.clone())}
#[tauri::command] fn workbook_put_batch(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,updates:std::collections::BTreeMap<String,String>)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.put_batch(expected_revision,sheet,updates)?;Ok(d.clone())}
#[tauri::command] fn workbook_rename_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize,name:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.rename_sheet(expected_revision,index,name)?;Ok(d.clone())}
#[tauri::command] fn workbook_delete_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.delete_sheet(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn workbook_value(s:State<'_,Workspace>,sheet:usize,address:String)->Result<String,String>{lock(&s.sheets)?.value(sheet,&address)}
#[tauri::command] fn workbook_find(s:State<'_,Workspace>,needle:String)->Result<Vec<suite::CellMatch>,String>{lock(&s.sheets)?.find_cells(&needle,100)}
#[tauri::command] fn workbook_statistics(s:State<'_,Workspace>,sheet:usize)->Result<suite::SheetStatistics,String>{lock(&s.sheets)?.statistics(sheet)}
#[tauri::command] fn workbook_open(s:State<'_,Workspace>,path:String)->Result<SheetBook,String>{let d:SheetBook=suite::load_json(Path::new(&path))?;d.validate()?;*lock(&s.sheets)?=d.clone();Ok(d)}
#[tauri::command] fn workbook_save(s:State<'_,Workspace>,path:String)->Result<(),String>{let d=lock(&s.sheets)?;d.validate()?;suite::save_json(Path::new(&path),&*d)}

#[tauri::command] fn deck_snapshot(s:State<'_,Workspace>)->Result<Deck,String>{Ok(lock(&s.slides)?.clone())}
#[tauri::command] fn deck_new(s:State<'_,Workspace>)->Result<Deck,String>{let mut d=lock(&s.slides)?;*d=Deck::blank();Ok(d.clone())}
#[tauri::command] fn deck_add(s:State<'_,Workspace>,expected_revision:u64)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.add(expected_revision)?;Ok(d.clone())}
#[tauri::command] fn deck_update(s:State<'_,Workspace>,expected_revision:u64,index:usize,slide:Slide)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.update(expected_revision,index,slide)?;Ok(d.clone())}
#[tauri::command] fn deck_delete(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.delete(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn deck_duplicate(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.duplicate(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn deck_move(s:State<'_,Workspace>,expected_revision:u64,from:usize,to:usize)->Result<Deck,String>{let mut d=lock(&s.slides)?;d.move_slide(expected_revision,from,to)?;Ok(d.clone())}
#[tauri::command] fn deck_open(s:State<'_,Workspace>,path:String)->Result<Deck,String>{let d:Deck=suite::load_json(Path::new(&path))?;d.validate()?;*lock(&s.slides)?=d.clone();Ok(d)}
#[tauri::command] fn deck_save(s:State<'_,Workspace>,path:String)->Result<(),String>{let d=lock(&s.slides)?;d.validate()?;suite::save_json(Path::new(&path),&*d)}

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
    *lock(&s.sheets)?=book.clone();
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
pub fn run(){tauri::Builder::default().plugin(tauri_plugin_dialog::init()).manage(Workspace{writer:Mutex::new(Document::blank()),sheets:Mutex::new(SheetBook::blank()),slides:Mutex::new(Deck::blank()),base:Mutex::new(Database::default())}).invoke_handler(tauri::generate_handler![snapshot,new_document,edit_document,rename_document,open_document,save_document,writer_embed_sheet,writer_embedded_put,writer_remove_embedded,writer_embedded_value,workbook_snapshot,workbook_new,workbook_put,workbook_add_sheet,workbook_put_batch,workbook_rename_sheet,workbook_delete_sheet,workbook_value,workbook_find,workbook_statistics,workbook_open,workbook_save,deck_snapshot,deck_new,deck_add,deck_update,deck_delete,deck_duplicate,deck_move,deck_open,deck_save,base_path,base_create,base_open,base_tables,base_create_table,base_insert,base_update,base_delete,base_browse,base_select,base_import_csv,base_indexes,base_create_index,base_delete_index,base_browse_page,base_to_workbook,base_integrity_check,base_backup,base_export_table_csv,base_group_report,automation_snapshot,automation_execute]).run(tauri::generate_context!()).expect("Failed to start Koffice")}