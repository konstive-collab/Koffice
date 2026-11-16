//! Higher-level spreadsheet operations (single atomic revision per operation).
//! No formatting metadata, hidden rows or merged cells exist in koffice.sheets.v1.
use crate::suite::SheetBook;
use std::collections::BTreeMap;

fn col_number(s:&str)->Result<usize,String>{
    if s.is_empty()||s.len()>3||!s.bytes().all(|b|b.is_ascii_uppercase()){
        return Err("Invalid column identifier".into());
    }
    let mut n=0usize;
    for b in s.bytes(){n=n*26+usize::from(b-b'A'+1);}
    if n>702{return Err("Column outside supported range".into());}
    Ok(n)
}
fn column(mut n:usize)->String{
    let mut s=String::new();
    while n>0{n-=1;s.insert(0,(b'A'+(n%26) as u8) as char);n/=26;}
    s
}
fn cell(row:usize,col:usize)->String{format!("{}{}",column(col),row)}
#[derive(Clone,Copy)]
struct Rect{top:usize,left:usize,bottom:usize,right:usize}
impl Rect{
    fn parse(range:&str,max_cells:usize)->Result<Self,String>{
        let (a,b)=range.split_once(':').ok_or("Expected a rectangular range such as A1:D20")?;
        fn parse_cell(v:&str)->Result<(usize,usize),String>{
            if v.is_empty()||v.len()>8{return Err("Invalid cell address".into());}
            let letters=v.bytes().take_while(|b|b.is_ascii_uppercase()).count();
            let col=col_number(&v[..letters])?;
            let row=v[letters..].parse::<usize>().map_err(|_|"Invalid row")?;
            if row==0||row>10_000{return Err("Row outside supported range".into());}
            Ok((row,col))
        }
        let (top,left)=parse_cell(a)?;
        let (bottom,right)=parse_cell(b)?;
        if top>bottom||left>right{return Err("Range must go from upper-left to lower-right".into());}
        let height=bottom-top+1;
        let width=right-left+1;
        if height.checked_mul(width).ok_or("Range too large")?>max_cells{return Err("Selected range is too large".into());}
        Ok(Self{top,left,bottom,right})
    }
    fn width(self)->usize{self.right-self.left+1}
    fn height(self)->usize{self.bottom-self.top+1}
}
fn sheet<'a>(book:&'a SheetBook,index:usize)->Result<&'a BTreeMap<String,String>,String>{
    Ok(&book.sheets.get(index).ok_or("Sheet not found")?.cells)
}
fn apply(book:&mut SheetBook,expected:u64,index:usize,changes:BTreeMap<String,String>)->Result<(),String>{
    book.put_batch(expected,index,changes)
}
/// Sorts complete rows inside a selected bounded rectangle by a column in it.
/// Formulas in the rectangle are rejected: moving formulas without a reference
/// rebasing model would silently change their meaning.
pub fn sort_range(book:&mut SheetBook,expected:u64,index:usize,range:&str,by:&str,ascending:bool,header:bool)->Result<(),String>{
    if book.revision!=expected{return Err("Workbook revision conflict".into());}
    let rect=Rect::parse(range,20_000)?;
    if rect.width()>30||rect.height()>2000{return Err("Sort supports at most 30 columns and 2,000 rows".into());}
    let keycol=col_number(by)?;
    if keycol<rect.left||keycol>rect.right{return Err("Sort key column must be within the range".into());}
    let cells=sheet(book,index)?;
    let start=rect.top+usize::from(header);
    if start>rect.bottom{return Err("No data rows to sort".into());}
    let mut rows:Vec<(usize,Vec<String>)>=(start..=rect.bottom).map(|row|{
        (row,(rect.left..=rect.right).map(|col|cells.get(&cell(row,col)).cloned().unwrap_or_default()).collect())
    }).collect();
    if rows.iter().flat_map(|(_,row)|row.iter()).any(|value|value.starts_with('=')){
        return Err("Cannot sort formulas safely. Copy or convert formulas to values first".into());
    }
    let offset=keycol-rect.left;
    rows.sort_by(|a,b|{
        let a=&a.1[offset];let b=&b.1[offset];
        let comparison=match (a.parse::<f64>(),b.parse::<f64>()){
            (Ok(x),Ok(y)) if x.is_finite()&&y.is_finite()=>x.total_cmp(&y),
            _=>a.to_lowercase().cmp(&b.to_lowercase()),
        };
        if ascending{comparison}else{comparison.reverse()}
    });
    let mut updates=BTreeMap::new();
    for (position,(_,values)) in rows.iter().enumerate(){
        for (i,value) in values.iter().enumerate(){
            updates.insert(cell(start+position,rect.left+i),value.clone());
        }
    }
    apply(book,expected,index,updates)
}
/// Fill an entire row or column with a numeric arithmetic progression.
pub fn fill_series(book:&mut SheetBook,expected:u64,index:usize,range:&str,start:f64,step:f64)->Result<(),String>{
    if book.revision!=expected{return Err("Workbook revision conflict".into());}
    let rect=Rect::parse(range,1000)?;
    if rect.width()!=1&&rect.height()!=1{return Err("Series must be one row or one column".into());}
    if !start.is_finite()||!step.is_finite(){return Err("Series must use finite numbers".into());}
    sheet(book,index)?;
    let mut updates=BTreeMap::new();
    for i in 0..rect.width()*rect.height(){
        let n=start+step*i as f64;
        if !n.is_finite(){return Err("Series value overflow".into());}
        updates.insert(cell(rect.top+if rect.width()==1{i}else{0},rect.left+if rect.width()>1{i}else{0}),n.to_string());
    }
    apply(book,expected,index,updates)
}
/// Clear bounded cell contents. Blank sparse entries remain absent.
pub fn clear_range(book:&mut SheetBook,expected:u64,index:usize,range:&str)->Result<(),String>{
    if book.revision!=expected{return Err("Workbook revision conflict".into());}
    let rect=Rect::parse(range,20_000)?;
    let cells=sheet(book,index)?;
    let mut updates=BTreeMap::new();
    for (address,_) in cells{
        let letters=address.bytes().take_while(|b|b.is_ascii_uppercase()).count();
        if let (Ok(col),Ok(row))=(col_number(&address[..letters]),address[letters..].parse::<usize>()){
            if (rect.top..=rect.bottom).contains(&row)&&(rect.left..=rect.right).contains(&col){
                updates.insert(address.clone(),String::new());
            }
        }
    }
    // Empty selection is a valid no-op; revision stays unchanged.
    if updates.is_empty(){return Ok(());}
    apply(book,expected,index,updates)
}
/// Replace a literal substring in non-formula cells within a bounded range.
/// No regex and no case folding: replacement is deterministic for Unicode and
/// never rewrites formula references. A zero-match operation is a true no-op.
pub fn replace_text(book:&mut SheetBook,expected:u64,index:usize,range:&str,needle:&str,replacement:&str)->Result<usize,String>{
    if book.revision!=expected{return Err("Workbook revision conflict".into());}
    if needle.is_empty()||needle.chars().count()>256||replacement.chars().count()>256{
        return Err("Replacement requires 1–256 search characters and at most 256 replacement characters".into());
    }
    let rect=Rect::parse(range,20_000)?;
    let cells=sheet(book,index)?;
    let mut updates=BTreeMap::new();
    let mut replacements=0usize;
    for (address,value) in cells{
        if value.starts_with('=')||!value.contains(needle){continue;}
        let letters=address.bytes().take_while(|b|b.is_ascii_uppercase()).count();
        let (Ok(col),Ok(row))=(col_number(&address[..letters]),address[letters..].parse::<usize>())else{continue;};
        if !(rect.top..=rect.bottom).contains(&row)||!(rect.left..=rect.right).contains(&col){continue;}
        let count=value.matches(needle).count();
        let changed=value.replace(needle,replacement);
        if changed!=*value{
            replacements=replacements.checked_add(count).ok_or("Replacement overflow")?;
            updates.insert(address.clone(),changed);
        }
    }
    if updates.is_empty(){return Ok(0);}
    apply(book,expected,index,updates)?;
    Ok(replacements)
}
#[cfg(test)]
mod tests{
 use super::*;
 fn seed()->SheetBook{
  let mut book=SheetBook::blank();
  let mut values=BTreeMap::new();
  for (addr,value) in [("A1","Name"),("B1","Score"),("A2","Bob"),("B2","3"),("A3","Ada"),("B3","10"),("A4","Cal"),("B4","2")]{
   values.insert(addr.into(),value.into());
  }
  book.put_batch(0,0,values).unwrap();book
 }
 #[test]fn numeric_sort_preserves_headers_and_rows(){
  let mut book=seed();
  sort_range(&mut book,1,0,"A1:B4","B",true,true).unwrap();
  let c=&book.sheets[0].cells;
  assert_eq!(c["A1"],"Name");assert_eq!(c["A2"],"Cal");assert_eq!(c["A3"],"Bob");assert_eq!(c["A4"],"Ada");
  assert_eq!(c["B2"],"2");assert_eq!(c["B4"],"10");
  assert_eq!(book.revision,2);
 }
 #[test]fn sort_is_atomic_rejects_formulas_and_wrong_revisions(){
  let mut book=seed();book.put(1,0,"B4".into(),"=B2*2".into()).unwrap();
  let old=book.clone();
  assert!(sort_range(&mut book,2,0,"A1:B4","B",true,true).is_err());
  assert!(sort_range(&mut book,1,0,"A1:B4","B",true,true).is_err());
  assert_eq!(book,old);
 }
 #[test]fn fill_series_supports_rows_columns_and_rejects_nonfinite(){
  let mut book=SheetBook::blank();
  fill_series(&mut book,0,0,"C2:C5",1.5,0.5).unwrap();
  assert_eq!(book.sheets[0].cells["C5"],"3");
  fill_series(&mut book,1,0,"D7:F7",10.0,-2.0).unwrap();
  assert_eq!(book.sheets[0].cells["F7"],"6");
  let before=book.clone();
  assert!(fill_series(&mut book,2,0,"A1:B2",0.0,1.0).is_err());
  assert!(fill_series(&mut book,2,0,"A1:A2",f64::NAN,1.0).is_err());
  assert_eq!(book,before);
 }
 #[test]fn clearing_range_preserves_outside_cells_and_is_atomic(){
  let mut book=seed();clear_range(&mut book,1,0,"A2:B3").unwrap();
  let c=&book.sheets[0].cells;
  assert!(!c.contains_key("A2"));assert!(!c.contains_key("B3"));
  assert_eq!(c["A4"],"Cal");
  let before=book.clone();
  assert!(clear_range(&mut book,1,0,"A2:B3").is_err());
  assert_eq!(book,before);
 }
 #[test]fn replacement_respects_bounds_and_preserves_formulas(){
  let mut b=seed();
  b.put(1,0,"C2".into(),"=B2*2".into()).unwrap();
  b.put(2,0,"A5".into(),"Bob outside".into()).unwrap();
  let count=replace_text(&mut b,3,0,"A2:B4","Bob","Robert").unwrap();
  assert_eq!(count,1);assert_eq!(b.sheets[0].cells["A2"],"Robert");
  assert_eq!(b.sheets[0].cells["A5"],"Bob outside");
  assert_eq!(b.sheets[0].cells["C2"],"=B2*2");
  let revision=b.revision;
  assert_eq!(replace_text(&mut b,revision,0,"A2:B4","Missing","Test").unwrap(),0);
  assert_eq!(b.revision,revision);
 }
 #[test]fn replacement_is_atomic_on_bad_limits_and_revision(){
  let mut b=seed();let before=b.clone();
  for range in ["A1:A30000","A1:Z1000","C4:A1"] {
   assert!(replace_text(&mut b,1,0,range,"a","b").is_err());
  }
  assert!(replace_text(&mut b,0,0,"A1:B4","a","b").is_err());
  assert!(replace_text(&mut b,1,0,"A1:B4","","b").is_err());
  assert_eq!(b,before);
 }
 #[test]fn invalid_and_huge_ranges_cannot_modify_data(){
  let mut book=seed();let old=book.clone();
  for range in ["B3:A2","A0:B2","A1:AAA10000","A1:A10001","A1:Z9000","A1"]{
   assert!(clear_range(&mut book,1,0,range).is_err(),"{range}");
  }
  assert_eq!(book,old);
 }
}
