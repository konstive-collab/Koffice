//! Koffice Fusion v0.15: persistent, manually refreshed Base -> Slides links.
//! Source files are never read when opening a .kslide. Inspect and refresh are
//! invoked only after the user explicitly approves the stored source path.
use crate::base::Database;
use crate::fusion_transfer::base_snapshot_slide;
use crate::suite::{BaseSlideSource,Deck,Slide,SlideContent};
use serde::Serialize;
use std::path::Path;

#[derive(Clone,Debug,Serialize)]
pub struct BaseSlideLinkStatus {
    pub state:String,
    pub detail:String,
    pub database_path:String,
    pub table:String,
    pub differing_fields:Vec<String>,
}

fn source_path(path:&str)->Result<String,String>{
    if !path.ends_with(".kdb") || path.len()>4096 {return Err("Expected a .kdb database".into());}
    let canonical=std::fs::canonicalize(path).map_err(|e|format!("Database path unavailable: {e}"))?;
    if !canonical.is_file() || canonical.extension().and_then(|s|s.to_str())!=Some("kdb") {
        return Err("The database source must be a .kdb file".into());
    }
    canonical.to_str().map(str::to_owned).ok_or_else(||"Database path is not valid Unicode".into())
}
fn snapshot(path:&str,table:&str)->Result<Slide,String>{
    let mut db=Database::default();
    db.open(path)?;
    let data=db.browse_page(table,None,0)?;
    base_snapshot_slide(table,&data)
}
fn checked_slide(deck:&Deck,index:usize)->Result<(&Slide,&BaseSlideSource),String>{
    let slide=deck.slides.get(index).ok_or("Slide not found")?;
    let link=slide.base_source.as_ref().ok_or("Slide has no linked Base source")?;
    Ok((slide,link))
}
fn fields_changed(a:&SlideContent,b:&SlideContent)->Vec<String>{
    let mut changed=Vec::new();
    if a.title!=b.title {changed.push("title".to_string());}
    if a.body!=b.body {changed.push("body".to_string());}
    if a.notes!=b.notes {changed.push("notes".to_string());}
    changed
}
pub fn create(deck:&mut Deck,expected_revision:u64,path:&str,table:&str)->Result<(),String>{
    if deck.revision!=expected_revision {return Err("Presentation revision conflict".into());}
    let path=source_path(path)?;
    let mut new_slide=snapshot(&path,table)?;
    let clean=new_slide.content();
    new_slide.base_source=Some(BaseSlideSource{database_path:path,table:table.to_string(),clean});
    let mut next=deck.clone();
    next.slides.push(new_slide);
    next.revision=next.revision.checked_add(1).ok_or("Presentation revision overflow")?;
    next.validate()?;
    *deck=next;
    Ok(())
}
pub fn inspect(deck:&Deck,index:usize)->Result<BaseSlideLinkStatus,String>{
    let (slide,link)=checked_slide(deck,index)?;
    let (state,detail,differing_fields)=if slide.content()!=link.clean {
        ("conflict","This slide was edited locally. Refresh is blocked; detach to keep the edits.",fields_changed(&slide.content(),&link.clean))
    } else {
        match snapshot(&link.database_path,&link.table){
            Err(_)=>("disconnected","Database or table unavailable. Original slide is preserved.",Vec::new()),
            Ok(new)=>{
                let differences=fields_changed(&slide.content(),&new.content());
                if differences.is_empty(){
                    ("current","Slide content matches the first page of the database table.",differences)
                }else{
                    ("changed","Source data changed. Refresh would replace this unchanged linked slide.",differences)
                }
            }
        }
    };
    Ok(BaseSlideLinkStatus{
        state:state.to_string(),detail:detail.to_string(),database_path:link.database_path.clone(),
        table:link.table.clone(),differing_fields,
    })
}
pub fn refresh(deck:&mut Deck,expected_revision:u64,index:usize)->Result<bool,String>{
    if deck.revision!=expected_revision {return Err("Presentation revision conflict".into());}
    let mut next=deck.clone();
    let slide=next.slides.get_mut(index).ok_or("Slide not found")?;
    let original=slide.base_source.clone().ok_or("Slide has no Base link")?;
    if slide.content()!=original.clean {
        return Err("Slide contains local changes; refresh would overwrite them. Detach first.".into());
    }
    let updated=snapshot(&original.database_path,&original.table)?;
    if updated.content()==slide.content() {return Ok(false);}
    let clean=updated.content();
    slide.title=updated.title;
    slide.body=updated.body;
    slide.notes=updated.notes;
    slide.base_source=Some(BaseSlideSource{clean,..original});
    next.revision=next.revision.checked_add(1).ok_or("Presentation revision overflow")?;
    next.validate()?;
    *deck=next;
    Ok(true)
}
/// Reconnect a moved .kdb using the explicit native file picker in the UI.
/// The table name is retained; local edits are never overwritten.
pub fn reconnect(deck:&mut Deck,expected_revision:u64,index:usize,new_path:&str)->Result<(),String>{
    if deck.revision!=expected_revision{return Err("Presentation revision conflict".into());}
    let mut next=deck.clone();
    let slide=next.slides.get_mut(index).ok_or("Slide not found")?;
    let original=slide.base_source.clone().ok_or("Slide has no linked Base source")?;
    if slide.content()!=original.clean {
        return Err("Slide has local edits; detach to preserve them before reconnecting".into());
    }
    let canonical=source_path(new_path)?;
    let updated=snapshot(&canonical,&original.table)?;
    let clean=updated.content();
    slide.title=updated.title;
    slide.body=updated.body;
    slide.notes=updated.notes;
    slide.base_source=Some(BaseSlideSource{database_path:canonical,table:original.table,clean});
    next.revision=next.revision.checked_add(1).ok_or("Presentation revision overflow")?;
    next.validate()?;
    *deck=next;Ok(())
}

pub fn detach(deck:&mut Deck,expected_revision:u64,index:usize)->Result<(),String>{
    if deck.revision!=expected_revision {return Err("Presentation revision conflict".into());}
    let mut next=deck.clone();
    let slide=next.slides.get_mut(index).ok_or("Slide not found")?;
    if slide.base_source.is_none(){return Err("Slide is not linked to Base".into());}
    slide.base_source=None;
    next.revision=next.revision.checked_add(1).ok_or("Presentation revision overflow")?;
    next.validate()?;
    *deck=next;
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::base::Field;
    use serde_json::{json,Map};
    use std::sync::atomic::{AtomicU64,Ordering};
    static SEQ:AtomicU64=AtomicU64::new(1);
    struct TempDb(String);
    impl TempDb {
        fn create()->Self{
            let filename=std::env::temp_dir().join(format!("koffice-base-slide-{}-{}.kdb",std::process::id(),SEQ.fetch_add(1,Ordering::Relaxed)));
            let file=Self(filename.to_str().unwrap().to_string());
            let mut db=Database::default();
            db.create(&file.0).unwrap();
            db.create_table("Sales",vec![Field{name:"amount".into(),kind:"INTEGER".into(),required:false}]).unwrap();
            file
        }
        fn insert(&self,amount:i64){
            let mut db=Database::default();db.open(&self.0).unwrap();
            let mut fields=Map::new();fields.insert("amount".into(),json!(amount));
            db.insert("Sales",fields).unwrap();
        }
    }
    impl Drop for TempDb{fn drop(&mut self){let _=std::fs::remove_file(&self.0);}}
    #[test]fn base_link_roundtrip_after_restart_and_manual_refresh(){
        let db=TempDb::create();db.insert(10);
        let mut deck=Deck::blank();create(&mut deck,0,&db.0,"Sales").unwrap();
        assert_eq!(deck.slides.len(),2);
        assert_eq!(inspect(&deck,1).unwrap().state,"current");
        let bytes=serde_json::to_vec(&deck).unwrap();
        let mut reopened:Deck=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reopened,deck);
        db.insert(20);
        assert_eq!(inspect(&reopened,1).unwrap().state,"changed");
        assert!(refresh(&mut reopened,1,1).unwrap());
        assert!(reopened.slides[1].body.contains("20"));
        assert_eq!(inspect(&reopened,1).unwrap().state,"current");
    }
    #[test]fn local_edits_block_refresh_and_keep_source(){
        let db=TempDb::create();db.insert(10);
        let mut deck=Deck::blank();create(&mut deck,0,&db.0,"Sales").unwrap();
        let mut modified=deck.slides[1].clone();modified.body="My edits".into();
        deck.update(1,1,modified).unwrap();
        assert!(deck.slides[1].base_source.is_some());
        db.insert(20);
        assert_eq!(inspect(&deck,1).unwrap().state,"conflict");
        let before=deck.clone();
        assert!(refresh(&mut deck,2,1).is_err());
        assert_eq!(deck,before);
        detach(&mut deck,2,1).unwrap();
        assert_eq!(deck.slides[1].body,"My edits");
        assert!(deck.slides[1].base_source.is_none());
    }
    #[test]fn move_duplicate_and_delete_keep_link_on_its_slide(){
        let db=TempDb::create();
        let mut deck=Deck::blank();create(&mut deck,0,&db.0,"Sales").unwrap();
        deck.duplicate(1,1).unwrap();
        assert!(deck.slides[2].base_source.is_some());
        deck.move_slide(2,2,0).unwrap();
        assert_eq!(inspect(&deck,0).unwrap().state,"current");
        deck.delete(3,1).unwrap();
        assert!(deck.slides[0].base_source.is_some());
    }
    #[test]fn stale_revision_and_missing_source_do_not_mutate_presentation(){
        let db=TempDb::create();
        let mut deck=Deck::blank();create(&mut deck,0,&db.0,"Sales").unwrap();
        let before=deck.clone();
        assert!(refresh(&mut deck,0,1).is_err());
        assert_eq!(deck,before);
        std::fs::remove_file(&db.0).unwrap();
        assert_eq!(inspect(&deck,1).unwrap().state,"disconnected");
        assert!(refresh(&mut deck,1,1).is_err());
        assert_eq!(deck,before);
    }

    #[test]fn moved_database_can_be_relinked_after_explicit_selection(){
        let db=TempDb::create();db.insert(1);
        let replacement=TempDb::create();replacement.insert(99);
        let mut deck=Deck::blank();create(&mut deck,0,&db.0,"Sales").unwrap();
        reconnect(&mut deck,1,1,&replacement.0).unwrap();
        assert!(deck.slides[1].body.contains("99"));
        assert_eq!(deck.slides[1].base_source.as_ref().unwrap().database_path,source_path(&replacement.0).unwrap());
        assert_eq!(inspect(&deck,1).unwrap().state,"current");
        let mut local=deck.slides[1].clone();local.body.push_str(" handwritten");
        deck.update(2,1,local).unwrap();
        let unchanged=deck.clone();
        assert!(reconnect(&mut deck,3,1,&db.0).is_err());
        assert_eq!(deck,unchanged);
    }
    #[test]fn legacy_slide_json_remains_loadable(){
        let legacy=r#"{"format":"koffice.slides.v1","title":"Old","revision":0,"slides":[{"title":"One","body":"Hello","notes":""}]}"#;
        let deck:Deck=serde_json::from_str(legacy).unwrap();
        assert!(deck.slides[0].base_source.is_none());
        deck.validate().unwrap();
    }
}
