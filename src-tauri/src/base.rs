//! Koffice Base: typed, parameterized SQLite operations shared by UI and future AI tools.
//! Native .kdb files are SQLite databases, not proprietary JSON snapshots.
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_ROWS: usize = 500;
const MAX_COLUMNS: usize = 64;
const MAX_IMPORT_BYTES: usize = 5_000_000;
const MAX_IMPORT_ROWS: usize = 10_000;

#[derive(Clone, Serialize, Deserialize)]
pub struct Field { pub name: String, pub kind: String, pub required: bool }
#[derive(Clone, Serialize, Deserialize)]
pub struct TableInfo { pub name: String, pub count: i64, pub fields: Vec<Field> }
#[derive(Clone, Serialize, Deserialize)]
pub struct QueryResult { pub columns: Vec<String>, pub rows: Vec<Vec<serde_json::Value>>, pub truncated: bool }
#[derive(Clone, Serialize, Deserialize)]
pub struct IndexInfo { pub name: String, pub unique: bool, pub columns: Vec<String> }
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportResult { pub inserted: usize, pub columns: Vec<String> }

pub struct Database { path: Option<PathBuf> }
impl Default for Database { fn default() -> Self { Self { path: None } } }

fn valid_name(s: &str) -> Result<(), String> {
    if s.len() > 64 || s.is_empty() || !s.bytes().next().unwrap().is_ascii_alphabetic()
        || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err("Identifiers must be 1-64 ASCII letters, digits or underscores, beginning with a letter".into());
    }
    if s.starts_with("sqlite_") { return Err("Reserved SQLite name".into()); }
    Ok(())
}
fn quote(s: &str) -> String { format!("\"{s}\"") } // valid_name checked at every call site
fn db_error(e: rusqlite::Error) -> String { format!("Database error: {e}") }
fn open_connection(path: &Path) -> Result<Connection, String> {
    if !path.is_file() { return Err("Database file does not exist".into()); }
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(db_error)?;
    conn.busy_timeout(std::time::Duration::from_secs(3)).map_err(db_error)?;
    conn.pragma_update(None, "foreign_keys", "ON").map_err(db_error)?;
    Ok(conn)
}
fn normalized_path(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if p.extension().and_then(|s| s.to_str()) != Some("kdb") { return Err("Use the .kdb extension".into()); }
    if !p.parent().is_some_and(|d| d.is_dir()) { return Err("Parent directory does not exist".into()); }
    Ok(p.to_path_buf())
}
fn json_to_sql(v: &serde_json::Value) -> Result<Value, String> {
    match v { serde_json::Value::Null => Ok(Value::Null), serde_json::Value::Bool(b) => Ok(Value::Integer(*b as i64)),
        serde_json::Value::Number(n) => if let Some(i)=n.as_i64(){Ok(Value::Integer(i))}else if let Some(f)=n.as_f64(){if f.is_finite(){Ok(Value::Real(f))}else{Err("Invalid number".into())}}else{Err("Invalid number".into())},
        serde_json::Value::String(s) => {if s.len()>1_000_000{Err("Cell exceeds 1 MB".into())}else{Ok(Value::Text(s.clone()))}},
        _ => Err("Only scalar JSON values are supported".into()) }
}
fn sql_to_json(value: rusqlite::types::ValueRef<'_>) -> serde_json::Value {
    use rusqlite::types::ValueRef;
    match value { ValueRef::Null=>serde_json::Value::Null, ValueRef::Integer(i)=>serde_json::json!(i),ValueRef::Real(f)=>serde_json::json!(f),ValueRef::Text(b)=>serde_json::Value::String(String::from_utf8_lossy(b).to_string()), ValueRef::Blob(b)=>serde_json::json!(format!("[BLOB {} bytes]",b.len())) }
}
impl Database {
    pub fn path(&self) -> Option<String> { self.path.as_ref().map(|p| p.to_string_lossy().into_owned()) }
    fn conn(&self)->Result<Connection,String>{open_connection(self.path.as_ref().ok_or("Open or create a database first")?)}
    pub fn create(&mut self,path:&str)->Result<(),String>{
        let path=normalized_path(path)?;
        if path.exists(){return Err("Destination already exists; open it instead".into());}
        let conn=Connection::open_with_flags(&path,rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE|rusqlite::OpenFlags::SQLITE_OPEN_CREATE).map_err(db_error)?;
        conn.pragma_update(None,"application_id", 0x4b4f4646_i64).map_err(db_error)?;
        conn.pragma_update(None,"foreign_keys","ON").map_err(db_error)?;
        drop(conn);self.path=Some(path);Ok(())
    }
    pub fn open(&mut self,path:&str)->Result<(),String>{
        let path=normalized_path(path)?;
        let conn=open_connection(&path)?;
        let check:String=conn.query_row("PRAGMA quick_check",[],|r|r.get(0)).map_err(db_error)?;
        if check!="ok"{return Err(format!("Database integrity check failed: {check}"));}
        drop(conn);self.path=Some(path);Ok(())
    }
    pub fn tables(&self)->Result<Vec<TableInfo>,String>{
        let conn=self.conn()?;
        let mut s=conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(db_error)?;
        let names=s.query_map([],|r|r.get::<_,String>(0)).map_err(db_error)?.collect::<Result<Vec<_>,_>>().map_err(db_error)?;
        let mut out=Vec::new();for name in names{if valid_name(&name).is_err(){continue;}let fields=self.fields_from(&conn,&name)?;
            let count:i64=conn.query_row(&format!("SELECT COUNT(*) FROM {}",quote(&name)),[],|r|r.get(0)).map_err(db_error)?;
            out.push(TableInfo{name,count,fields});}Ok(out)
    }
    fn fields_from(&self,conn:&Connection,table:&str)->Result<Vec<Field>,String>{
        valid_name(table)?;let mut s=conn.prepare(&format!("PRAGMA table_info({})",quote(table))).map_err(db_error)?;
        let fields=s.query_map([],|r|Ok(Field{name:r.get(1)?,kind:r.get(2)?,required:r.get::<_,i64>(3)?!=0})).map_err(db_error)?;
        fields.collect::<Result<Vec<_>,_>>().map_err(db_error)
    }
    pub fn create_table(&self,table:&str,fields:Vec<Field>)->Result<(),String>{
        valid_name(table)?;if fields.is_empty()||fields.len()>MAX_COLUMNS{return Err("Provide 1-64 fields".into());}
        let mut seen=std::collections::HashSet::new();let mut clauses=vec!["\"id\" INTEGER PRIMARY KEY AUTOINCREMENT".to_string()];
        for f in &fields {valid_name(&f.name)?;if f.name.eq_ignore_ascii_case("id")||!seen.insert(f.name.to_ascii_lowercase()){return Err("Duplicate or reserved field name".into());}
            let kind=f.kind.to_ascii_uppercase();if !["TEXT","INTEGER","REAL","BLOB"].contains(&kind.as_str()){return Err("Field type must be TEXT, INTEGER, REAL or BLOB".into());}
            clauses.push(format!("{} {}{}",quote(&f.name),kind,if f.required{" NOT NULL"}else{""}));}
        self.conn()?.execute(&format!("CREATE TABLE {} ({})",quote(table),clauses.join(", ")),[]).map_err(db_error)?;Ok(())
    }
    pub fn insert(&self,table:&str,values:serde_json::Map<String,serde_json::Value>)->Result<i64,String>{
        valid_name(table)?;if values.is_empty()||values.len()>MAX_COLUMNS{return Err("Provide 1-64 field values".into());}
        let conn=self.conn()?;let allowed=self.fields_from(&conn,table)?.into_iter().map(|f|f.name).collect::<Vec<_>>();
        let mut cols=Vec::new();let mut vals=Vec::new();for (k,v) in values {valid_name(&k)?;if k=="id"||!allowed.contains(&k){return Err(format!("Unknown or protected column: {k}"));}cols.push(quote(&k));vals.push(json_to_sql(&v)?);}
        let placeholders=vec!["?";cols.len()].join(",");conn.execute(&format!("INSERT INTO {} ({}) VALUES ({})",quote(table),cols.join(","),placeholders),params_from_iter(vals)).map_err(db_error)?;Ok(conn.last_insert_rowid())
    }
    pub fn update(&self,table:&str,id:i64,values:serde_json::Map<String,serde_json::Value>)->Result<usize,String>{
        valid_name(table)?;if id<1||values.is_empty()||values.len()>MAX_COLUMNS{return Err("Invalid row ID or empty update".into());}
        let conn=self.conn()?;let allowed=self.fields_from(&conn,table)?.into_iter().map(|f|f.name).collect::<Vec<_>>();
        let mut sets=Vec::new();let mut vals=Vec::new();for(k,v)in values{valid_name(&k)?;if k=="id"||!allowed.contains(&k){return Err(format!("Unknown or protected column: {k}"));}sets.push(format!("{}=?",quote(&k)));vals.push(json_to_sql(&v)?);}
        vals.push(Value::Integer(id));conn.execute(&format!("UPDATE {} SET {} WHERE id=?",quote(table),sets.join(",")),params_from_iter(vals)).map_err(db_error)
    }
    pub fn delete(&self,table:&str,id:i64)->Result<usize,String>{valid_name(table)?;if id<1{return Err("Invalid row ID".into());}self.conn()?.execute(&format!("DELETE FROM {} WHERE id=?",quote(table)),params![id]).map_err(db_error)}
    pub fn browse(&self,table:&str,search:Option<String>)->Result<QueryResult,String>{
        valid_name(table)?;let conn=self.conn()?;let fields=self.fields_from(&conn,table)?;if fields.is_empty(){return Err("Table does not exist".into());}
        let search=search.unwrap_or_default();if search.len()>256{return Err("Search term is too long".into());}
        let pattern=format!("%{}%",search.replace('\\',"\\\\").replace('%',"\\%").replace('_',"\\_"));
        let sql=if search.is_empty(){format!("SELECT * FROM {} ORDER BY id LIMIT {}",quote(table),MAX_ROWS+1)}else{
            let filters=fields.iter().filter(|f|f.kind.eq_ignore_ascii_case("TEXT")).map(|f|format!("{} LIKE ? ESCAPE '\\'",quote(&f.name))).collect::<Vec<_>>();
            if filters.is_empty(){format!("SELECT * FROM {} ORDER BY id LIMIT {}",quote(table),MAX_ROWS+1)}else{
                let sql=format!("SELECT * FROM {} WHERE {} ORDER BY id LIMIT {}",quote(table),filters.join(" OR "),MAX_ROWS+1);
                return query_conn(&conn,&sql,&vec![Value::Text(pattern);filters.len()]);}
        };
        query_conn(&conn,&sql,&[])
    }
    /// Import a CSV snapshot to an existing table; any invalid row rolls back the entire batch.
    pub fn import_csv(&self, table:&str, content:&str)->Result<ImportResult,String>{
        valid_name(table)?;
        if content.len()>MAX_IMPORT_BYTES { return Err("CSV exceeds 5 MB".into()); }
        let mut conn=self.conn()?;
        let allowed=self.fields_from(&conn,table)?;
        if allowed.is_empty(){return Err("Table does not exist".into());}
        let mut reader=csv::ReaderBuilder::new().flexible(false).from_reader(content.trim_start_matches('\u{feff}').as_bytes());
        let headers=reader.headers().map_err(|e|format!("Invalid CSV header: {e}"))?.iter().map(str::to_owned).collect::<Vec<_>>();
        if headers.is_empty()||headers.len()>MAX_COLUMNS{return Err("CSV must have 1-64 columns".into());}
        let mut seen=std::collections::HashSet::new();
        for col in &headers {valid_name(col)?;if col.eq_ignore_ascii_case("id") || !seen.insert(col.to_ascii_lowercase()) || !allowed.iter().any(|f|f.name==*col){return Err(format!("Unknown, duplicate, or protected CSV field: {col}"));}}
        let placeholders=vec!["?";headers.len()].join(",");
        let sql=format!("INSERT INTO {} ({}) VALUES ({})",quote(table),headers.iter().map(|h|quote(h)).collect::<Vec<_>>().join(","),placeholders);
        let tx=conn.transaction().map_err(db_error)?;
        let mut count=0;
        {let mut stmt=tx.prepare(&sql).map_err(db_error)?;
            for record in reader.records(){
                if count>=MAX_IMPORT_ROWS {return Err("CSV import exceeds 10,000 records; no records were imported".into());}
                let record=record.map_err(|e|format!("CSV row {}: {e}",count+2))?;
                let mut values=Vec::with_capacity(headers.len());
                for (i,cell) in record.iter().enumerate(){
                    if cell.len()>1_000_000 {return Err(format!("CSV row {}: field exceeds 1 MB",count+2));}
                    let kind=&allowed.iter().find(|f|f.name==headers[i]).ok_or("CSV field missing")?.kind;
                    let value=if cell.is_empty(){Value::Null}else{match kind.to_ascii_uppercase().as_str(){
                        "INTEGER"=>Value::Integer(cell.parse::<i64>().map_err(|_|format!("CSV row {}: invalid INTEGER for {}",count+2,headers[i]))?),
                        "REAL"=>{let n=cell.parse::<f64>().map_err(|_|format!("CSV row {}: invalid REAL for {}",count+2,headers[i]))?;if !n.is_finite(){return Err("Non-finite value".into());}Value::Real(n)},
                        _=>Value::Text(cell.to_owned())
                    }};
                    values.push(value);
                }
                stmt.execute(params_from_iter(values)).map_err(|e|format!("CSV row {}: {e}",count+2))?;
                count+=1;
            }
        }
        tx.commit().map_err(db_error)?;
        Ok(ImportResult{inserted:count,columns:headers})
    }
    /// Only known table columns can be indexed; automatic index names cannot target internal SQLite indexes.
    pub fn create_index(&self,table:&str,columns:Vec<String>,unique:bool)->Result<String,String>{
        valid_name(table)?;
        if columns.is_empty()||columns.len()>4{return Err("Indexes require 1-4 columns".into());}
        let conn=self.conn()?;let fields=self.fields_from(&conn,table)?;
        if fields.is_empty(){return Err("Table does not exist".into());}
        let mut seen=std::collections::HashSet::new();
        for name in &columns{valid_name(name)?;if !fields.iter().any(|f|f.name==*name)||!seen.insert(name.to_ascii_lowercase()){return Err("Unknown or repeated index column".into());}}
        let name=format!("kidx_{}_{}",table,columns.join("_"));
        if name.len()>64{return Err("Index name is too long".into());}
        let sql=format!("CREATE {} INDEX {} ON {} ({})",if unique{"UNIQUE"}else{""},quote(&name),quote(table),columns.iter().map(|s|quote(s)).collect::<Vec<_>>().join(","));
        conn.execute(&sql,[]).map_err(db_error)?;
        Ok(name)
    }
    pub fn indexes(&self,table:&str)->Result<Vec<IndexInfo>,String>{
        valid_name(table)?;let conn=self.conn()?;
        if self.fields_from(&conn,table)?.is_empty(){return Err("Table does not exist".into());}
        let mut stmt=conn.prepare(&format!("PRAGMA index_list({})",quote(table))).map_err(db_error)?;
        let found=stmt.query_map([],|r|Ok((r.get::<_,String>(1)?,r.get::<_,i64>(2)?!=0))).map_err(db_error)?.collect::<Result<Vec<_>,_>>().map_err(db_error)?;
        let mut result=Vec::new();
        for (name,unique) in found{if !name.starts_with("kidx_"){continue;}
            // Index name comes from SQLite, not a user-supplied SQL fragment; quote by escaping.
            let mut cols=conn.prepare(&format!("PRAGMA index_info(\"{}\")",name.replace('"',"\"\""))).map_err(db_error)?;
            let columns=cols.query_map([],|r|r.get::<_,String>(2)).map_err(db_error)?.collect::<Result<Vec<_>,_>>().map_err(db_error)?;
            result.push(IndexInfo{name,unique,columns});
        }
        Ok(result)
    }
    pub fn delete_index(&self,name:&str)->Result<(),String>{
        valid_name(name)?;
        if !name.starts_with("kidx_"){return Err("Only Koffice-created indexes can be removed".into());}
        let conn=self.conn()?;
        let found:Option<String>=conn.query_row("SELECT name FROM sqlite_master WHERE type='index' AND name=?1",[name],|r|r.get(0)).optional().map_err(db_error)?;
        if found.is_none(){return Err("Index not found".into());}
        conn.execute(&format!("DROP INDEX {}",quote(name)),[]).map_err(db_error)?;Ok(())
    }
    /// Offset pagination for large databases: never loads the whole table into memory.
    pub fn browse_page(&self,table:&str,search:Option<String>,offset:usize)->Result<QueryResult,String>{
        if offset>1_000_000{return Err("Offset exceeds 1,000,000".into());}
        valid_name(table)?;let conn=self.conn()?;let fields=self.fields_from(&conn,table)?;
        if fields.is_empty(){return Err("Table does not exist".into());}
        let search=search.unwrap_or_default();if search.len()>256{return Err("Search term is too long".into());}
        let pattern=format!("%{}%",search.replace('\\',"\\\\").replace('%',"\\%").replace('_',"\\_"));
        let filters=fields.iter().filter(|f|f.kind.eq_ignore_ascii_case("TEXT")).map(|f|format!("{} LIKE ? ESCAPE '\\'",quote(&f.name))).collect::<Vec<_>>();
        let sql=if search.is_empty()||filters.is_empty(){format!("SELECT * FROM {} ORDER BY id LIMIT {} OFFSET {}",quote(table),MAX_ROWS+1,offset)}else{
            format!("SELECT * FROM {} WHERE {} ORDER BY id LIMIT {} OFFSET {}",quote(table),filters.join(" OR "),MAX_ROWS+1,offset)};
        let args=if search.is_empty()||filters.is_empty(){vec![]}else{vec![Value::Text(pattern);filters.len()]};
        query_conn(&conn,&sql,&args)
    }
    /// Run a consistency check and return a bounded result suitable for the UI and AI tools.
    pub fn integrity_check(&self)->Result<String,String>{
        let conn=self.conn()?;
        let result:String=conn.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(db_error)?;
        Ok(result)
    }
    /// Copy a live SQLite database consistently. Existing files are NEVER overwritten.
    /// SQLite VACUUM INTO makes a coherent snapshot even when the source uses WAL.
    pub fn backup(&self,destination:&str)->Result<(),String>{
        let dst=normalized_path(destination)?;
        if dst.exists(){return Err("Backup destination already exists".into());}
        if self.path.as_ref().is_some_and(|p|p == &dst){return Err("Cannot back up onto the active database".into());}
        let conn=self.conn()?;
        conn.execute("VACUUM INTO ?1",[dst.to_string_lossy().as_ref()]).map_err(db_error)?;
        Ok(())
    }
    /// Bounded, RFC-compatible CSV export of an entire table, not only the visible page.
    pub fn export_table_csv(&self,table:&str)->Result<String,String>{
        valid_name(table)?;
        let conn=self.conn()?;
        let fields=self.fields_from(&conn,table)?;
        if fields.is_empty(){return Err("Table does not exist".into());}
        let count:i64=conn.query_row(&format!("SELECT COUNT(*) FROM {}",quote(table)),[],|r|r.get(0)).map_err(db_error)?;
        if count>MAX_IMPORT_ROWS as i64{return Err("CSV export is limited to 10,000 records".into());}
        let mut statement=conn.prepare(&format!("SELECT * FROM {} ORDER BY id",quote(table))).map_err(db_error)?;
        let columns=statement.column_names().iter().map(|x|x.to_string()).collect::<Vec<_>>();
        let mut writer=csv::Writer::from_writer(Vec::new());
        writer.write_record(&columns).map_err(|e|e.to_string())?;
        let mut rows=statement.query([]).map_err(db_error)?;
        let mut exported=0usize;
        while let Some(row)=rows.next().map_err(db_error)? {
            exported+=1;
            if exported>MAX_IMPORT_ROWS{return Err("CSV export exceeds 10,000 records".into());}
            let cells=(0..columns.len()).map(|i|{
                let value=row.get_ref(i).map_err(db_error)?;
                Ok::<String,String>(match value{
                    rusqlite::types::ValueRef::Null=>String::new(),
                    rusqlite::types::ValueRef::Integer(n)=>n.to_string(),
                    rusqlite::types::ValueRef::Real(n)=>n.to_string(),
                    rusqlite::types::ValueRef::Text(t)=>String::from_utf8_lossy(t).into_owned(),
                    rusqlite::types::ValueRef::Blob(b)=>format!("[BLOB {} bytes]",b.len()),
                })
            }).collect::<Result<Vec<_>,_>>()?;
            writer.write_record(cells).map_err(|e|e.to_string())?;
        }
        String::from_utf8(writer.into_inner().map_err(|e|e.to_string())?).map_err(|e|e.to_string())
    }
    /// Read-only grouped report. The table and field names must exist in the schema.
    pub fn group_report(&self,table:&str,group_field:&str,metric_field:Option<&str>)->Result<QueryResult,String>{
        valid_name(table)?;valid_name(group_field)?;
        let conn=self.conn()?;
        let fields=self.fields_from(&conn,table)?;
        if !fields.iter().any(|f|f.name==group_field){return Err("Grouping field not found".into());}
        let mut columns=vec![format!("{} AS category",quote(group_field)),"COUNT(*) AS records".into()];
        if let Some(metric)=metric_field{
            valid_name(metric)?;
            let field=fields.iter().find(|f|f.name==metric).ok_or("Metric field not found")?;
            if !["INTEGER","REAL"].contains(&field.kind.to_ascii_uppercase().as_str()){
                return Err("Metrics require INTEGER or REAL fields".into());
            }
            columns.push(format!("SUM({}) AS total",quote(metric)));
            columns.push(format!("AVG({}) AS average",quote(metric)));
        }
        let sql=format!("SELECT {} FROM {} GROUP BY {} ORDER BY records DESC, category LIMIT {}",columns.join(","),quote(table),quote(group_field),MAX_ROWS+1);
        query_conn(&conn,&sql,&[])
    }
    /// SELECT/WITH only: rejects multiple statements and all writes, including writable CTEs.
    pub fn select(&self,sql:&str)->Result<QueryResult,String>{
        if sql.len()>32_768{return Err("SQL exceeds 32 KiB".into());}
        if sql.contains(';'){return Err("Multiple SQL statements and semicolons are not allowed".into());}
        let sql=sql.trim();let leading=sql.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        if leading!="SELECT"&&leading!="WITH"{return Err("Only read-only SELECT queries are allowed".into());}
        let conn=self.conn()?;let stmt=conn.prepare(sql).map_err(db_error)?;
        if !stmt.readonly(){return Err("Write queries are disabled in SQL explorer".into());}
        if stmt.parameter_count()>0{return Err("SQL explorer does not accept parameters".into());}
        drop(stmt);query_conn(&conn,sql,&[])
    }
}
fn query_conn(conn:&Connection,sql:&str,values:&[Value])->Result<QueryResult,String>{
    let mut stmt=conn.prepare(sql).map_err(db_error)?;if !stmt.readonly(){return Err("Read-only query expected".into());}
    let columns=stmt.column_names().iter().map(|s|s.to_string()).collect::<Vec<_>>();if columns.len()>MAX_COLUMNS{return Err("Result exceeds 64 columns".into());}
    let mut rows=stmt.query(params_from_iter(values.iter())).map_err(db_error)?;let mut out=Vec::new();while let Some(row)=rows.next().map_err(db_error)?{if out.len()==MAX_ROWS{return Ok(QueryResult{columns,rows:out,truncated:true});}
        let mut cells=Vec::new();for i in 0..columns.len(){cells.push(sql_to_json(row.get_ref(i).map_err(db_error)?));}out.push(cells);}
    Ok(QueryResult{columns,rows:out,truncated:false})
}

#[cfg(test)] mod tests {
    use super::*;
    fn temp()->(Database,PathBuf){let mut db=Database::default();let path=std::env::temp_dir().join(format!("koffice-base-{}-{}.kdb",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));db.create(path.to_str().unwrap()).unwrap();(db,path)}
    #[test] fn crud_and_reopen(){let(mut db,path)=temp();db.create_table("people",vec![Field{name:"name".into(),kind:"TEXT".into(),required:true},Field{name:"age".into(),kind:"INTEGER".into(),required:false}]).unwrap();let id=db.insert("people",serde_json::from_value(serde_json::json!({"name":"Анна","age":23})).unwrap()).unwrap();assert_eq!(db.tables().unwrap()[0].count,1);assert_eq!(db.browse("people",Some("Анна".into())).unwrap().rows.len(),1);assert_eq!(db.update("people",id,serde_json::from_value(serde_json::json!({"age":24})).unwrap()).unwrap(),1);db.open(path.to_str().unwrap()).unwrap();assert_eq!(db.select("SELECT age FROM people").unwrap().rows[0][0],24);assert_eq!(db.delete("people",id).unwrap(),1);std::fs::remove_file(path).unwrap();}
    #[test] fn rejects_sql_injection_and_bad_fields(){let(db,path)=temp();assert!(db.create_table("x; DROP TABLE y",vec![Field{name:"n".into(),kind:"TEXT".into(),required:false}]).is_err());db.create_table("safe",vec![Field{name:"value".into(),kind:"TEXT".into(),required:false}]).unwrap();assert!(db.select("DELETE FROM safe").is_err());assert!(db.select("WITH x AS (DELETE FROM safe RETURNING *) SELECT * FROM x").is_err());assert!(db.select("SELECT * FROM safe; DELETE FROM safe").is_err());assert!(db.insert("safe",serde_json::from_value(serde_json::json!({"value":"x", "evil":10})).unwrap()).is_err());assert_eq!(db.tables().unwrap()[0].count,0);std::fs::remove_file(path).unwrap();}
    #[test] fn csv_import_is_atomic_and_handles_quoted_fields(){
        let(db,path)=temp();db.create_table("people",vec![Field{name:"name".into(),kind:"TEXT".into(),required:true},Field{name:"age".into(),kind:"INTEGER".into(),required:false}]).unwrap();
        let csv="name,age\n\"Анна, И.\",24\nБорис,29\n";
        assert_eq!(db.import_csv("people",csv).unwrap().inserted,2);
        assert_eq!(db.browse("people",Some("Анна".into())).unwrap().rows[0][1],"Анна, И.");
        assert!(db.import_csv("people","name,age\nОлег,40\nМария,not_a_number\n").is_err());
        assert_eq!(db.tables().unwrap()[0].count,2);
        assert!(db.import_csv("people","id,name\n1,Evil\n").is_err());
        assert!(db.import_csv("people","name,name\na,b\n").is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test] fn indexes_and_pagination(){
        let(db,path)=temp();db.create_table("people",vec![Field{name:"name".into(),kind:"TEXT".into(),required:false}]).unwrap();
        let idx=db.create_index("people",vec!["name".into()],true).unwrap();
        assert_eq!(db.indexes("people").unwrap()[0].columns,vec!["name"]);
        db.import_csv("people","name\nAnn\nBob\n").unwrap();
        assert_eq!(db.browse_page("people",None,1).unwrap().rows.len(),1);
        assert!(db.import_csv("people","name\nAnn\n").is_err());
        assert_eq!(db.tables().unwrap()[0].count,2);
        assert!(db.delete_index("sqlite_autoindex_people_1").is_err());
        db.delete_index(&idx).unwrap();assert!(db.indexes("people").unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }
    #[test] fn backup_reports_and_full_export(){
        let(db,path)=temp();
        db.create_table("records",vec![Field{name:"category".into(),kind:"TEXT".into(),required:true},Field{name:"amount".into(),kind:"REAL".into(),required:false}]).unwrap();
        db.import_csv("records","category,amount\nКниги,12.5\nКниги,7.5\nКофе,3\n").unwrap();
        assert_eq!(db.integrity_check().unwrap(),"ok");
        let report=db.group_report("records","category",Some("amount")).unwrap();
        assert_eq!(report.rows[0][1],2);
        assert_eq!(report.rows[0][2],20.0);
        assert!(db.group_report("records","category",Some("category")).is_err());
        assert!(db.group_report("records","missing",None).is_err());
        let csv=db.export_table_csv("records").unwrap();
        assert!(csv.contains("Книги,12.5"));
        let destination=path.with_file_name(format!("backup-{}",path.file_name().unwrap().to_string_lossy()));
        db.backup(destination.to_str().unwrap()).unwrap();
        assert!(db.backup(destination.to_str().unwrap()).is_err());
        let mut restored=Database::default();restored.open(destination.to_str().unwrap()).unwrap();
        assert_eq!(restored.tables().unwrap()[0].count,3);
        std::fs::remove_file(destination).unwrap();std::fs::remove_file(path).unwrap();
    }
    #[test] fn pagination_cap(){let(db,path)=temp();db.create_table("items",vec![Field{name:"label".into(),kind:"TEXT".into(),required:false}]).unwrap();let conn=db.conn().unwrap();let tx=conn.unchecked_transaction().unwrap();for _ in 0..501{tx.execute("INSERT INTO items (label) VALUES ('x')",[]).unwrap();}tx.commit().unwrap();let r=db.browse("items",None).unwrap();assert_eq!(r.rows.len(),500);assert!(r.truncated);drop(conn);std::fs::remove_file(path).unwrap();}
}