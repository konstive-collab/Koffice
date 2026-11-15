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
pub struct Slide { pub title: String, pub body: String, pub notes: String }

impl SheetBook {
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
        if let Some(body) = expr.strip_prefix("SUM(").and_then(|s| s.strip_suffix(')')) {
            let (from, to) = body.split_once(':').ok_or("SUM requires a range")?;
            let (r1,c1) = parse_address(from)?; let (r2,c2) = parse_address(to)?;
            if r2 < r1 || c2 < c1 || (r2-r1+1)*(c2-c1+1) > 10_000 { return Err("SUM range is invalid or too large".into()); }
            let mut sum = 0f64;
            for r in r1..=r2 { for c in c1..=c2 {
                let key = format_address(r,c);
                sum += evaluate(cells, &key, stack, depth+1)?.parse::<f64>().unwrap_or(0.0);
            }}
            return Ok(format_number(sum));
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
    pub fn blank() -> Self { Self { format: "koffice.slides.v1".into(), title: "Untitled presentation".into(), revision: 0, slides: vec![Slide { title: "Title slide".into(), body: "Click to add content".into(), notes: String::new() }] } }
    pub fn validate(&self) -> Result<(),String> {
        if self.format != "koffice.slides.v1" { return Err("Unsupported presentation format".into()); }
        check_title(&self.title)?;
        if self.slides.is_empty() || self.slides.len()>MAX_SLIDES { return Err("Presentation requires 1–500 slides".into()); }
        for s in &self.slides { if s.title.len()>MAX_INPUT || s.body.len()>MAX_INPUT || s.notes.len()>MAX_INPUT { return Err("Slide text too long".into()); } }
        Ok(())
    }
    pub fn add(&mut self, expected:u64) -> Result<(),String> {
        check_revision(self.revision, expected)?; let mut copy=self.clone();
        copy.slides.push(Slide {title:format!("Slide {}",copy.slides.len()+1), body:String::new(), notes:String::new()});
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?; copy.validate()?; *self=copy; Ok(())
    }
    pub fn update(&mut self, expected:u64, index:usize, slide:Slide) -> Result<(),String> {
        check_revision(self.revision, expected)?; let mut copy=self.clone(); *copy.slides.get_mut(index).ok_or("Slide not found")?=slide;
        copy.revision=copy.revision.checked_add(1).ok_or("Revision overflow")?; copy.validate()?; *self=copy; Ok(())
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
 #[test] fn validation_rejects_duplicate_sheet_names(){let mut b=SheetBook::blank(); assert!(b.add_sheet(0,"sheet 1".into()).is_err());}
}