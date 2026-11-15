mod engine;
mod suite;
use engine::{Document, Edit};
use suite::{Deck, SheetBook, Slide};
use std::{path::Path, sync::Mutex};
use tauri::State;

struct Workspace { writer: Mutex<Document>, sheets: Mutex<SheetBook>, slides: Mutex<Deck> }
fn lock<T>(mutex:&Mutex<T>)->Result<std::sync::MutexGuard<'_,T>,String>{mutex.lock().map_err(|_|"Workspace lock poisoned".into())}

#[tauri::command] fn snapshot(s:State<'_,Workspace>)->Result<Document,String>{Ok(lock(&s.writer)?.clone())}
#[tauri::command] fn new_document(s:State<'_,Workspace>)->Result<Document,String>{let mut d=lock(&s.writer)?;*d=Document::blank();Ok(d.clone())}
#[tauri::command] fn edit_document(s:State<'_,Workspace>,expected_revision:u64,edit:Edit)->Result<Document,String>{let mut d=lock(&s.writer)?;d.apply(expected_revision,edit)?;Ok(d.clone())}
#[tauri::command] fn rename_document(s:State<'_,Workspace>,expected_revision:u64,title:String)->Result<Document,String>{let mut d=lock(&s.writer)?;if d.revision!=expected_revision{return Err("Revision conflict".into());}let mut next=d.clone();next.title=title;next.revision=next.revision.checked_add(1).ok_or("Revision overflow")?;next.validate()?;*d=next;Ok(d.clone())}
#[tauri::command] fn open_document(s:State<'_,Workspace>,path:String)->Result<Document,String>{let d=engine::open(Path::new(&path))?;*lock(&s.writer)?=d.clone();Ok(d)}
#[tauri::command] fn save_document(s:State<'_,Workspace>,path:String)->Result<(),String>{engine::save(Path::new(&path),&*lock(&s.writer)?)}

#[tauri::command] fn workbook_snapshot(s:State<'_,Workspace>)->Result<SheetBook,String>{Ok(lock(&s.sheets)?.clone())}
#[tauri::command] fn workbook_new(s:State<'_,Workspace>)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;*d=SheetBook::blank();Ok(d.clone())}
#[tauri::command] fn workbook_put(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,address:String,value:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.put(expected_revision,sheet,address,value)?;Ok(d.clone())}
#[tauri::command] fn workbook_add_sheet(s:State<'_,Workspace>,expected_revision:u64,name:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.add_sheet(expected_revision,name)?;Ok(d.clone())}
#[tauri::command] fn workbook_put_batch(s:State<'_,Workspace>,expected_revision:u64,sheet:usize,updates:std::collections::BTreeMap<String,String>)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.put_batch(expected_revision,sheet,updates)?;Ok(d.clone())}
#[tauri::command] fn workbook_rename_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize,name:String)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.rename_sheet(expected_revision,index,name)?;Ok(d.clone())}
#[tauri::command] fn workbook_delete_sheet(s:State<'_,Workspace>,expected_revision:u64,index:usize)->Result<SheetBook,String>{let mut d=lock(&s.sheets)?;d.delete_sheet(expected_revision,index)?;Ok(d.clone())}
#[tauri::command] fn workbook_value(s:State<'_,Workspace>,sheet:usize,address:String)->Result<String,String>{lock(&s.sheets)?.value(sheet,&address)}
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(){tauri::Builder::default().plugin(tauri_plugin_dialog::init()).manage(Workspace{writer:Mutex::new(Document::blank()),sheets:Mutex::new(SheetBook::blank()),slides:Mutex::new(Deck::blank())}).invoke_handler(tauri::generate_handler![snapshot,new_document,edit_document,rename_document,open_document,save_document,workbook_snapshot,workbook_new,workbook_put,workbook_add_sheet,workbook_put_batch,workbook_rename_sheet,workbook_delete_sheet,workbook_value,workbook_open,workbook_save,deck_snapshot,deck_new,deck_add,deck_update,deck_delete,deck_duplicate,deck_move,deck_open,deck_save]).run(tauri::generate_context!()).expect("Failed to start Koffice")}