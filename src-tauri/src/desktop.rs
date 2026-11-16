//! Koffice app separation: one install, four independent native processes.
//! CLI: koffice --app writer|sheets|slides|base
//! No shells, custom paths or arbitrary command execution.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum DesktopMode { Suite, Writer, Sheets, Slides, Base }
impl DesktopMode {
    pub fn as_str(self)->&'static str {
        match self {Self::Suite=>"suite",Self::Writer=>"writer",Self::Sheets=>"sheets",Self::Slides=>"slides",Self::Base=>"base"}
    }
    pub fn parse(value:&str)->Result<Self,String>{
        match value {
            "suite"=>Ok(Self::Suite),"writer"=>Ok(Self::Writer),
            "sheets"=>Ok(Self::Sheets),"slides"=>Ok(Self::Slides),"base"=>Ok(Self::Base),
            _=>Err("Unknown Koffice application".into())
        }
    }
    pub fn from_args<I,S>(args:I)->Self
    where I:IntoIterator<Item=S>,S:AsRef<str> {
        let mut args=args.into_iter();
        while let Some(part)=args.next() {
            if part.as_ref()=="--app" {
                return args.next().and_then(|s|Self::parse(s.as_ref()).ok()).unwrap_or(Self::Suite);
            }
        }
        Self::Suite
    }
    pub fn window_title(self)->&'static str {
        match self {
            Self::Suite=>"Koffice — Office Suite",
            Self::Writer=>"Kodi Writer — Document Editor",
            Self::Sheets=>"Kodi Sheets — Spreadsheet Editor",
            Self::Slides=>"Kodi Slides — Presentation Editor",
            Self::Base=>"Kodi Base — Database Editor",
        }
    }
}
#[tauri::command]
pub fn desktop_mode()->&'static str {
    DesktopMode::from_args(std::env::args().skip(1)).as_str()
}
#[tauri::command]
pub fn desktop_launch(kind:String)->Result<String,String>{
    let target=DesktopMode::parse(&kind)?;
    if target==DesktopMode::Suite{return Err("Launch a specific application".into());}
    let executable=std::env::current_exe().map_err(|e|format!("Cannot locate Koffice executable: {e}"))?;
    std::process::Command::new(executable)
        .arg("--app").arg(target.as_str())
        .spawn().map_err(|e|format!("Could not launch {}: {e}",target.window_title()))?;
    Ok(target.window_title().to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cli_modes_are_strict() {
        for (text,mode) in [
            ("suite",DesktopMode::Suite),("writer",DesktopMode::Writer),
            ("sheets",DesktopMode::Sheets),("slides",DesktopMode::Slides),
            ("base",DesktopMode::Base)]{
            assert_eq!(DesktopMode::parse(text).unwrap(),mode);
            assert_eq!(DesktopMode::from_args(["--app",text]),mode);
        }
        for invalid in ["", "terminal","--help","WRITER","../../bin/sh","writer;calc","writer\nother"]{
            assert!(DesktopMode::parse(invalid).is_err());
        }
        assert_eq!(DesktopMode::from_args(["--anything"]),DesktopMode::Suite);
        assert_eq!(DesktopMode::from_args(["--app"]),DesktopMode::Suite);
    }
    #[test] fn every_app_has_a_distinct_native_window_title() {
        let mut names=std::collections::HashSet::new();
        for app in [DesktopMode::Writer,DesktopMode::Sheets,DesktopMode::Slides,DesktopMode::Base] {
            assert!(names.insert(app.window_title()));
        }
    }
}
