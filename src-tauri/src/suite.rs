//! AI-ready typed operations for tables and presentations.
//! The desktop UI and future RPC/MCP adapters share these functions.
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, path::Path};

const MAX_CELLS: usize = 100_000;
const MAX_SLIDES: usize = 500;
const MAX_INPUT: usize = 16_384;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SheetBook {
    pub format: String,
    pub title: String,
    pub revision: u64,
    pub sheets: Vec<Sheet>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sheet {
    pub name: String,
    pub cells: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Deck {
    pub format: String,
    pub title: String,
    pub revision: u64,
    pub slides: Vec<Slide>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SlideContent { pub title: String, pub body: String, pub notes: String }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaseSlideSource {
    pub database_path: String,
    pub table: String,
    pub clean: SlideContent,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Slide {
    pub title: String,
    pub body: String,
    pub notes: String,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub base_source: Option<BaseSlideSource>,
}
impl Slide {
    pub fn content(&self)->SlideContent{
        SlideContent{title:self.title.clone(),body:self.body.clone(),notes:self.notes.clone()}
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CellMatch { pub sheet: usize, pub address: String, pub value: String }

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SheetStatistics {
    pub populated: usize,
    pub numeric: usize,
    pub formulas: usize,
    pub sum: f64,
    pub average: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

impl SheetBook {
    /// Non-destructive, case-insensitive search across all sheets.
    pub fn find_cells(&self, needle: &str, limit: usize) -> Result<Vec<CellMatch>, String> {
        if needle.trim().is_empty() || needle.chars().count() > 256 || limit == 0 || limit > 500 {
            return Err("Search needs 1–256 characters and a result limit of 1–500".into());
        }
        let needle = needle.to_lowercase();
        let mut matches = Vec::new();
        for (sheet, tab) in self.sheets.iter().enumerate() {
            for (address, value) in &tab.cells {
                if value.to_lowercase().contains(&needle) {
                    matches.push(CellMatch { sheet, address: address.clone(), value: value.clone() });
                    if matches.len() == limit { return Ok(matches); }
                }
            }
        }
        Ok(matches)
    }
    /// Read-only numerical profile of one sheet; formula errors are ignored as nonnumeric.
    pub fn statistics(&self, sheet: usize) -> Result<SheetStatistics, String> {
        let tab = self.sheets.get(sheet).ok_or("Sheet not found")?;
        let mut n = 0usize;
        let mut sum = 0.0_f64;
        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;
        let mut formulas = 0;
        for (address, raw) in &tab.cells {
            if raw.starts_with('=') { formulas += 1; }
            if let Ok(number) = self.value(sheet, address).unwrap_or_default().parse::<f64>() {
                if number.is_finite() {
                    sum += number;
                    n += 1;
                    min = Some(min.map_or(number, |m| m.min(number)));
                    max = Some(max.map_or(number, |m| m.max(number)));
                }
            }
        }
        Ok(SheetStatistics {
            populated: tab.cells.len(), numeric: n, formulas,
            sum, average: (n > 0).then_some(sum / n.max(1) as f64),
            minimum: min, maximum: max,
        })
    }
    pub fn blank() -> Self { Self { format: "koffice.sheets.v1".into(), title: "Untitled workbook".into(), revision: 0, sheets: vec![Sheet { name: "Sheet 1".into(), cells: BTreeMap::new() }] } }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "koffice.sheets.v1" { return Err("Unsupported workbook format".into()); }
        check_title(&self.title)?;
        if self.sheets.is_empty() || self.sheets.len() > 50 { return Err("Workbook must have 1–50 sheets".into()); }
        let mut names = BTreeSet::new();
        let mut count = 0;
        for sheet in &self.sheets {
            check_title(&sheet.name)?;
            if !names.insert(sheet.name.to_lowercase()) { return Err("Duplicate sheet name".into()); }
            count += sheet.cells.len();
            if count > MAX_CELLS { return Err("Too many cells".into()); }
            for (address, value) in &sheet.cells { parse_address(address)?; if value.len() > MAX_INPUT { return Err("Cell content too long".into()); } }
        }
        Ok(())
    }
    pub fn put(&mut self, expected: u64, sheet: usize, address: String, value: String) -> Result<(), String> {
        check_revision(self.revision, expected)?;
        parse_address(&address)?;
        if value.len() > MAX_INPUT { return Err("Cell content too long".into()); }
        let mut copy = self.clone();
        let tab = copy.sheets.get_mut(sheet).ok_or("Sheet not found")?;
        if value.is_empty() { tab.cells.remove(&address); } else { tab.cells.insert(address, value); }
        copy.bump()?; copy.validate()?; *self = copy; Ok(())
    }
    pub fn add_sheet(&mut self, expected: u64, name: String) -> Result<(), String> {
        check_revision(self.revision, expected)?;
        let mut copy = self.clone();
        copy.sheets.push(Sheet { name, cells: BTreeMap::new() });
        copy.bump()?; copy.validate()?; *self = copy; Ok(())
    }
    /// Atomic multi-cell edit, including clipboard paste. A single revision is committed.
    pub fn put_batch(&mut self, expected: u64, sheet: usize, updates: BTreeMap<String, String>) -> Result<(), String> {
        check_revision(self.revision, expected)?;
        if updates.len() > 20_000 { return Err("Batch exceeds 20,000 cells".into()); }
        let mut copy = self.clone();
        let tab = copy.sheets.get_mut(sheet).ok_or("Sheet not found")?;
        for (address, value) in updates {
            parse_address(&address)?;
            if value.len() > MAX_INPUT { return Err("Cell content too long".into()); }
            if value.is_empty() { tab.cells.remove(&address); } else { tab.cells.insert(address, value); }
        }
        copy.bump()?; copy.validate()?; *self = copy; Ok(())
    }
    pub fn rename_sheet(&mut self, expected: u64, index: usize, name: String) -> Result<(), String> {
        check_revision(self.revision, expected)?;
        let mut copy = self.clone();
        copy.sheets.get_mut(index).ok_or("Sheet not found")?.name = name;
        copy.bump()?; copy.validate()?; *self = copy; Ok(())
    }
    pub fn delete_sheet(&mut self, expected: u64, index: usize) -> Result<(), String> {
        check_revision(self.revision, expected)?;
        if self.sheets.len() == 1 { return Err("Cannot delete the last sheet".into()); }
        let mut copy = self.clone();
        if index >= copy.sheets.len() { return Err("Sheet not found".into()); }
        copy.sheets.remove(index);
        copy.bump()?; copy.validate()?; *self = copy; Ok(())
    }
    fn bump(&mut self) -> Result<(), String> { self.revision = self.revision.checked_add(1).ok_or("Revision overflow")?; Ok(()) }
    /// Preview formula values without mutating the saved source expression.
    pub fn value(&self, sheet: usize, address: &str) -> Result<String, String> {
        let cells = &self.sheets.get(sheet).ok_or("Sheet not found")?.cells;
        let mut stack = BTreeSet::new();
        let n = evaluate(cells, address, &mut stack, 0)?;
        Ok(n)
    }
}

fn evaluate(cells: &BTreeMap<String, String>, address: &str, stack: &mut BTreeSet<String>, depth: usize) -> Result<String, String> {
    parse_address(address)?;
    if depth > 32 { return Err("Formula nesting limit".into()); }
    if !stack.insert(address.to_owned()) { return Err("Circular reference".into()); }
    let result = (|| -> Result<String,String> {
        let value = cells.get(address).map(String::as_str).unwrap_or("");
        let Some(expr) = value.strip_prefix('=') else { return Ok(value.to_owned()); };
        let expr = expr.trim().to_uppercase();
        for func in ["SUM", "AVERAGE", "COUNT", "MIN", "MAX"] {
          if let Some(body) = expr.strip_prefix(func).and_then(|s| s.strip_prefix('(')).and_then(|s| s.strip_suffix(')')) {
            let (from, to) = body.split_once(':').ok_or("Function requires a range")?;
            let (r1,c1) = parse_address(from)?; let (r2,c2) = parse_address(to)?;
            if r2 < r1 || c2 < c1 || (r2-r1+1)*(c2-c1+1) > 10_000 { return Err("Formula range is invalid or too large".into()); }
            let mut nums=Vec::new();
            for r in r1..=r2 { for c in c1..=c2 {
                let key=format_address(r,c);
                let val=evaluate(cells,&key,stack,depth+1)?;
                if let Ok(num)=val.parse::<f64>() { if num.is_finite() { nums.push(num); } }
            }}
            let answer=match func {
                "SUM"=>nums.iter().sum(),
                "COUNT"=>nums.len() as f64,
                "AVERAGE"=>{if nums.is_empty(){return Err("AVERAGE of empty range".into());}nums.iter().sum::<f64>()/nums.len() as f64},
                "MIN"=>*nums.iter().reduce(|a,b|if a<b{a}else{b}).ok_or("MIN of empty range")?,
                "MAX"=>*nums.iter().reduce(|a,b|if a>b{a}else{b}).ok_or("MAX of empty range")?,
                _=>unreachable!()
            };
            return Ok(format_number(answer));
          }
        }
        if expr.chars().all(|c| c.is_ascii_alphanumeric() || c == '$') && expr.chars().any(|c| c.is_ascii_alphabetic()) {
            let ref_addr = expr.replace('$', "");
            parse_address(&ref_addr)?;
            return evaluate(cells, &ref_addr, stack, depth+1);
        }
        // Arithmetic expressions support numeric constants and + - * /, with standard precedence.
        let mut parser = Parser { bytes: expr.as_bytes(), pos: 0 };
        let n = parser.expr()?; parser.ws();
        if parser.pos != parser.bytes.len() || !n.is_finite() { return Err("Invalid formula".into()); }
        Ok(format_number(n))
    })();
    stack.remove(address);
    result
}
fn format_number(n: f64) -> String { if n.fract() == 0.0 { format!("{n:.0}") } else { n.to_string() } }
struct Parser<'a> { bytes: &'a [u8], pos: usize }
impl Parser<'_> {
    fn ws(&mut self) { while self.bytes.get(self.pos) == Some(&b' ') { self.pos += 1; } }
    fn expr(&mut self) -> Result<f64, String> { let mut a = self.term()?; loop { self.ws(); match self.bytes.get(self.pos) { Some(b'+') => { self.pos+=1; a+=self.term()?; }, Some(b'-') => { self.pos+=1; a-=self.term()?; }, _=>return Ok(a) } } }
    fn term(&mut self) -> Result<f64, String> { let mut a = self.atom()?; loop { self.ws(); match self.bytes.get(self.pos) { Some(b'*') => { self.pos+=1; a*=self.atom()?; }, Some(b'/') => { self.pos+=1; let b=self.atom()?; if b==0.0 { return Err("Division by zero".into()); } a/=b; }, _=>return Ok(a) } } }
    fn atom(&mut self) -> Result<f64, String> {
        self.ws(); if self.bytes.get(self.pos)==Some(&b'-') { self.pos+=1; return Ok(-self.atom()?); }
        if self.bytes.get(self.pos)==Some(&b'(') { self.pos+=1; let n=self.expr()?; self.ws(); if self.bytes.get(self.pos)!=Some(&b')') { return Err("Missing ')'".into()); } self.pos+=1; return Ok(n); }
        let start=self.pos;
        while self.bytes.get(self.pos).is_some_and(|b| b.is_ascii_digit() || *b==b'.') { self.pos+=1; }
        if start==self.pos { return Err("Expected number".into()); }
        std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e|e.to_string())?.parse::<f64>().map_err(|_|"Invalid number".into())
    }
}
fn parse_address(a: &str) -> Result<(usize,usize),String> {
    if a.len()>8 || a.is_empty() { return Err("Invalid cell address".into()); }
    let letters = a.bytes().take_while(|b| b.is_ascii_uppercase()).count();
    if letters == 0 || letters > 3 || letters == a.len() { return Err("Invalid cell address".into()); }
    let mut col=0usize;
    for b in a.bytes().take(letters) { col=col*26+usize::from(b-b'A'+1); }
    let row=a[letters..].parse::<usize>().map_err(|_|"Invalid cell row")?;
    if row==0 || row>10_000 || col>702 { return Err("Cell outside supported grid".into()); }
    Ok((row,col))
}
fn format_address(row:usize, mut col:usize)->String { let mut s=String::new(); while col>0 { col-=1; s.insert(0, (b'A'+(col%26) as u8) as char); col/=26; } format!("{s}{row}") }

impl Deck {
    pub fn blank() -> Self { Self { format: "koffice.slides.v1".into(), title: "Untitled presentation".into(), revision: 0, slides: vec![Slide { title: "Title slide".into(), body: "Click to add content".into(), notes: String::new(), base_source: None }] } }
    pub fn validate(&self) -> Result<(),String> {
        if self.format != "koffice.slides.v1" { return Err("Unsupported presentation format".into()); }
        check_title(&self.title)?;
        if self.slides.is_empty() || self.slides.len()>MAX_SLIDES { return Err("Presentation requires 1–500 slides".into()); }
        for s in &self.slides {
            if s.title.len()>MAX_INPUT || s.body.len()>MAX_INPUT || s.notes.len()>MAX_INPUT { return Err("Slide text too long".into()); }
            if let Some(link)=&s.base_source {
                if link.database_path.is_empty() || link.database_path.len()>4096 || !link.database_path.ends_with(".kdb")
                    || link.table.is_empty() || link.table.len()>64 ||
                    !link.table.bytes().next().unwrap().is_ascii_alphabetic() ||
                    !link.table.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_') {
                    return Err("Invalid linked Base source".into());
                }
                let clean=&link.clean;
                if clean.title.len()>MAX_INPUT || clean.body.len()>MAX_INPUT || clean.notes.len()>MAX_INPUT {
                    return Err("Linked Base snapshot exceeds slide limits".into());
                }
            }
        }
        Ok(())
    }
    pub fn add(&mut self, expected:u64) -> Result<(),String> {
        check_revision(self.revision, expected)?; let mut copy=self.clone();
        copy.slides.push(Slide {title:format!("Slide {}",copy.slides.len()+1), body:String::new(), notes:String::new(), base_source: None});
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?; copy.validate()?; *self=copy; Ok(())
    }
    pub fn update(&mut self, expected:u64, index:usize, slide:Slide) -> Result<(),String> {
        check_revision(self.revision, expected)?; let mut copy=self.clone();
        let old=copy.slides.get(index).ok_or("Slide not found")?;
        // Preserve source metadata during ordinary text editing. Only a dedicated
        // detach command may remove the link. Never trust metadata from UI payload.
        let mut replacement=slide;
        replacement.base_source=old.base_source.clone();
        copy.slides[index]=replacement;
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?; copy.validate()?; *self=copy; Ok(())
    }
    pub fn duplicate(&mut self, expected:u64, index:usize) -> Result<(),String> {
        check_revision(self.revision, expected)?;
        let mut copy=self.clone();
        let slide=copy.slides.get(index).ok_or("Slide not found")?.clone();
        copy.slides.insert(index+1,slide);
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?;
        copy.validate()?; *self=copy; Ok(())
    }
    pub fn move_slide(&mut self, expected:u64, from:usize, to:usize) -> Result<(),String> {
        check_revision(self.revision,expected)?;
        if from>=self.slides.len() || to>=self.slides.len(){return Err("Slide not found".into());}
        let mut copy=self.clone();
        let item=copy.slides.remove(from);copy.slides.insert(to,item);
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?;
        copy.validate()?;*self=copy;Ok(())
    }
    pub fn delete(&mut self, expected:u64, index:usize) -> Result<(),String> {
        check_revision(self.revision, expected)?; let mut copy=self.clone(); if copy.slides.len()==1 {return Err("Cannot remove the last slide".into());}
        if index>=copy.slides.len() {return Err("Slide not found".into());} copy.slides.remove(index);
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?; copy.validate()?; *self=copy; Ok(())
    }
}
fn check_title(title:&str)->Result<(),String> { if title.trim().is_empty() || title.chars().count()>200 { Err("Title must have 1–200 characters".into()) } else {Ok(())} }
fn check_revision(current:u64,expected:u64)->Result<(),String> { if current!=expected {Err("Revision conflict".into())} else {Ok(())} }

pub fn load_json<T:for<'de> Deserialize<'de>>(path:&Path)->Result<T,String> {
    let m=std::fs::metadata(path).map_err(|e|e.to_string())?;
    if m.len()>32*1024*1024 { return Err("File too large".into()); }
    let b=std::fs::read(path).map_err(|e|e.to_string())?;
    serde_json::from_slice(&b).map_err(|e|e.to_string())
}
pub fn save_json<T:Serialize>(path:&Path,object:&T)->Result<(),String> {
    // Reuse the same checked atomic file-replacement path as Writer.
    let bytes=serde_json::to_vec_pretty(object).map_err(|e|e.to_string())?;
    crate::engine::save_bytes(path,&bytes)
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn sums_cells_and_preserves_formula() {let mut b=SheetBook::blank(); b.put(0,0,"A1".into(),"12".into()).unwrap(); b.put(1,0,"A2".into(),"8".into()).unwrap(); b.put(2,0,"B1".into(),"=SUM(A1:A2)".into()).unwrap(); assert_eq!(b.value(0,"B1").unwrap(),"20"); assert_eq!(b.sheets[0].cells["B1"],"=SUM(A1:A2)");}
 #[test] fn circular_and_out_of_bounds() {let mut b=SheetBook::blank(); assert!(b.put(0,0,"A0".into(),"x".into()).is_err()); b.put(0,0,"A1".into(),"=B1".into()).unwrap(); b.put(1,0,"B1".into(),"=A1".into()).unwrap(); assert!(b.value(0,"A1").is_err());}
 #[test] fn arithmetic_precedence() {let mut b=SheetBook::blank();b.put(0,0,"C3".into(),"=2+3*4".into()).unwrap(); assert_eq!(b.value(0,"C3").unwrap(),"14");}
 #[test] fn slide_revision_and_delete() {let mut d=Deck::blank();d.add(0).unwrap();assert!(d.delete(0,0).is_err());d.delete(1,0).unwrap();assert_eq!(d.slides.len(),1);}
 #[test] fn batch_is_atomic_and_checks_revision() {
   let mut b=SheetBook::blank(); let mut updates=BTreeMap::new();updates.insert("A1".into(),"4".into());updates.insert("ZZZ999".into(),"bad".into());
   assert!(b.put_batch(0,0,updates).is_err());assert_eq!(b.revision,0);assert!(b.sheets[0].cells.is_empty());
   let mut valid=BTreeMap::new();valid.insert("A1".into(),"4".into());valid.insert("A2".into(),"6".into());
   b.put_batch(0,0,valid).unwrap();assert_eq!(b.revision,1);assert_eq!(b.value(0,"A2").unwrap(),"6");
   assert!(b.put_batch(0,0,BTreeMap::new()).is_err());
 }
 #[test] fn aggregates_ignore_nonnumeric_and_reject_empty_average(){let mut b=SheetBook::blank();b.put(0,0,"A1".into(),"3".into()).unwrap();b.put(1,0,"A2".into(),"7".into()).unwrap();b.put(2,0,"A3".into(),"hello".into()).unwrap();for (i,(formula,result)) in [("=AVERAGE(A1:A3)","5"),("=COUNT(A1:A3)","2"),("=MIN(A1:A3)","3"),("=MAX(A1:A3)","7")].iter().enumerate(){let address=format!("B{}",i+1);b.put(b.revision,0,address.clone(),formula.to_string()).unwrap();assert_eq!(b.value(0,&address).unwrap(),*result);}b.put(b.revision,0,"C1".into(),"=AVERAGE(D1:D2)".into()).unwrap();assert!(b.value(0,"C1").is_err());}
 #[test] fn sheet_management_and_slide_reorder(){let mut b=SheetBook::blank();b.add_sheet(0,"Second".into()).unwrap();b.rename_sheet(1,1,"Renamed".into()).unwrap();assert_eq!(b.sheets[1].name,"Renamed");b.delete_sheet(2,0).unwrap();assert!(b.delete_sheet(3,0).is_err());let mut d=Deck::blank();d.add(0).unwrap();d.duplicate(1,0).unwrap();assert_eq!(d.slides.len(),3);d.move_slide(2,0,2).unwrap();assert_eq!(d.slides[2].title,"Title slide");}
 #[test] fn validation_rejects_duplicate_sheet_names(){let mut b=SheetBook::blank(); assert!(b.add_sheet(0,"sheet 1".into()).is_err());}
 #[test] fn search_matches_without_mutation() {
   let mut b=SheetBook::blank();b.put(0,0,"A1".into(),"Привет".into()).unwrap();b.put(1,0,"B2".into(),"hello".into()).unwrap();
   let before=b.clone();let found=b.find_cells("ПРИ",10).unwrap();
   assert_eq!(found.len(),1);assert_eq!(found[0].address,"A1");assert_eq!(b,before);
   assert!(b.find_cells("",10).is_err());
 }
 #[test] fn statistics_include_numeric_formula_values() {
   let mut b=SheetBook::blank();b.put(0,0,"A1".into(),"10".into()).unwrap();b.put(1,0,"A2".into(),"20".into()).unwrap();
   b.put(2,0,"B1".into(),"=SUM(A1:A2)".into()).unwrap();b.put(3,0,"C1".into(),"text".into()).unwrap();
   let r=b.statistics(0).unwrap();assert_eq!((r.populated,r.numeric,r.formulas),(4,3,1));
   assert_eq!(r.sum,60.0);assert_eq!(r.average,Some(20.0));
 }

}