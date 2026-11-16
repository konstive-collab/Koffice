//! Standalone MCP stdio bridge. This is an isolated, in-memory session and never
//! accesses the desktop application's live workspace or arbitrary filesystem paths.
#[path = "../engine.rs"] mod engine;
#[path = "../suite.rs"] mod suite;
#[path = "../automation.rs"] mod automation;
#[path = "../fusion.rs"] mod fusion;

use std::io::{self, BufRead, Read, Write};
use serde_json::{json, Value};
use engine::Document;
use suite::{Deck, SheetBook};
use automation::{AutomationBatch, AutomationSnapshot};

const PROTOCOL: &str = "2025-06-18";
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

struct Session { writer: Document, sheets: SheetBook, slides: Deck }
impl Session {
    fn new() -> Self { Self { writer: Document::blank(), sheets: SheetBook::blank(), slides: Deck::blank() } }
    fn snapshot(&self) -> Value {
        serde_json::to_value(AutomationSnapshot {
            writer: self.writer.clone(), sheets: self.sheets.clone(), slides: self.slides.clone()
        }).expect("valid in-memory snapshot")
    }
}

fn tool_list() -> Value {
    json!([
        {
            "name": "koffice_snapshot",
            "description": "Read the current isolated Koffice MCP session: Writer, Sheets, Slides, and revisions.",
            "inputSchema": {"type":"object","properties":{},"additionalProperties":false}
        },
        {
            "name": "koffice_execute",
            "description": "Apply 1-50 revision-checked Writer/Sheets/Slides changes atomically in this isolated MCP session. Changes are not saved to disk or reflected in desktop Koffice.",
            "inputSchema": {
                "type":"object",
                "properties":{"operations":{"type":"array","minItems":1,"maxItems":50,"items":{"type":"object"}}},
                "required":["operations"],"additionalProperties":false
            }
        },
        {
            "name": "koffice_reset",
            "description": "Discard the in-memory MCP session and start blank Writer, Sheets and Slides workspaces. This is irreversible within the session.",
            "inputSchema":{"type":"object","properties":{},"additionalProperties":false}
        }
    ])
}

fn rpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn tool_result(data: Value, is_error: bool) -> Value {
    json!({"content":[{"type":"text","text":serde_json::to_string(&data).unwrap_or_else(|_| "{}".into())}],"isError":is_error})
}
fn process(session: &mut Session, request: Value) -> Option<Value> {
    let object=match request.as_object() {
        Some(obj)=>obj,
        None=>return Some(rpc_error(Value::Null,-32600,"Request must be an object")),
    };
    let id=object.get("id").cloned();
    // All notifications are silent, including notifications/initialized and cancellation.
    let id=match id {Some(x) if x.is_string() || x.is_number()=>x,Some(Value::Null)=>Value::Null,Some(_)=>return Some(rpc_error(Value::Null,-32600,"Invalid request id")),None=>return None};
    if object.get("jsonrpc").and_then(Value::as_str)!=Some("2.0") {
        return Some(rpc_error(id,-32600,"Expected JSON-RPC 2.0"));
    }
    let method=match object.get("method").and_then(Value::as_str) {
        Some(m)=>m,None=>return Some(rpc_error(id,-32600,"Missing method")),
    };
    let params=object.get("params").cloned().unwrap_or_else(||json!({}));
    let result=match method {
        "initialize"=>json!({"protocolVersion":PROTOCOL,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"koffice-mcp","version":"0.8.0"}}),
        "ping"=>json!({}),
        "tools/list"=>json!({"tools":tool_list()}),
        "tools/call"=>{
            let name=params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments=params.get("arguments").cloned().unwrap_or_else(||json!({}));
            match name {
                "koffice_snapshot" if arguments==json!({})=>tool_result(session.snapshot(),false),
                "koffice_execute"=>{
                    match serde_json::from_value::<AutomationBatch>(arguments) {
                        Ok(batch)=>match automation::execute(&mut session.writer,&mut session.sheets,&mut session.slides,batch) {
                            Ok(result)=>tool_result(json!({"result":result,"snapshot":session.snapshot()}),false),
                            Err(err)=>tool_result(json!({"error":err}),true),
                        },
                        Err(err)=>tool_result(json!({"error":format!("Invalid batch: {err}")}),true)
                    }
                },
                "koffice_reset" if arguments==json!({})=>{
                    *session=Session::new();
                    tool_result(session.snapshot(),false)
                },
                "koffice_snapshot" | "koffice_reset"=>tool_result(json!({"error":"Unexpected arguments"}),true),
                _=>return Some(rpc_error(id,-32602,"Unknown tool name")),
            }
        }
        _=>return Some(rpc_error(id,-32601,"Method not found")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

fn main() {
    let stdin=io::stdin();
    let mut input=stdin.lock();
    let stdout=io::stdout();
    let mut output=stdout.lock();
    let mut session=Session::new();
    loop {
        let mut line=Vec::new();
        // Limit input BEFORE allocation to avoid unbounded memory from untrusted agents.
        let length=match input.by_ref().take((MAX_REQUEST_BYTES+2) as u64).read_until(b'\n',&mut line) {
            Ok(n)=>n, Err(e)=>{eprintln!("MCP stdin error: {e}");break;}
        };
        if length==0 {break;}
        if length>MAX_REQUEST_BYTES || !line.ends_with(b"\n") {
            // Oversize or malformed framing is terminal: do not attempt to resync.
            let err=rpc_error(Value::Null,-32600,"MCP request too large or missing newline");
            let _=writeln!(output,"{err}");
            break;
        }
        if line.iter().all(u8::is_ascii_whitespace){continue;}
        let response=match serde_json::from_slice::<Value>(&line) {
            Ok(req)=>process(&mut session,req),
            Err(e)=>Some(rpc_error(Value::Null,-32700,&format!("Invalid JSON: {e}"))),
        };
        if let Some(value)=response {
            if writeln!(output,"{value}").is_err() || output.flush().is_err(){break;}
        }
    }
}

#[cfg(test)]
mod mcp_tests {
    use super::*;
    fn call(s:&mut Session,id:i32,name:&str,args:Value)->Value{
        process(s,json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})).unwrap()
    }
    #[test] fn discovery_and_notifications() {
        let mut s=Session::new();
        assert!(process(&mut s,json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
        let init=process(&mut s,json!({"jsonrpc":"2.0","id":1,"method":"initialize"})).unwrap();
        assert_eq!(init["result"]["protocolVersion"],PROTOCOL);
        let tools=process(&mut s,json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(),3);
    }
    #[test] fn update_and_rollback() {
        let mut s=Session::new();
        let good=call(&mut s,1,"koffice_execute",json!({"operations":[{"type":"writer_edit","expected_revision":0,"edit":{"type":"replace_all","text":"Hello"}},{"type":"sheets_put_batch","expected_revision":0,"sheet":0,"updates":{"A1":"12"}}]}));
        assert_eq!(good["result"]["isError"],false);
        assert_eq!(s.writer.text,"Hello");
        assert_eq!(s.sheets.sheets[0].cells["A1"],"12");
        let failed=call(&mut s,2,"koffice_execute",json!({"operations":[{"type":"writer_edit","expected_revision":1,"edit":{"type":"replace_all","text":"gone"}},{"type":"sheets_put_batch","expected_revision":1,"sheet":0,"updates":{"A0":"bad"}}]}));
        assert_eq!(failed["result"]["isError"],true);
        assert_eq!(s.writer.text,"Hello");
        assert_eq!(s.sheets.sheets[0].cells["A1"],"12");
    }
    #[test] fn rejects_unknown_tools_and_params() {
        let mut s=Session::new();
        assert_eq!(call(&mut s,1,"execute_shell",json!({}))["error"]["code"],-32602);
        assert_eq!(call(&mut s,2,"koffice_snapshot",json!({"path":"/etc/passwd"}))["result"]["isError"],true);
    }
}
