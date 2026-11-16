# Koffice v0.8 — MCP & Python automation (experimental)

Koffice v0.8 adds an **MCP stdio server** written in Rust and a dependency-free Python SDK. Both call the **same document operations** used by the desktop Automation API. The MCP server is a separate, isolated, in-memory session; it does **not** access currently opened desktop tabs, databases, files, or the network.

## Build and run

Install the Rust toolchain and platform prerequisites for the Koffice Tauri project, then:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --bin koffice-mcp
cargo test --manifest-path src-tauri/Cargo.toml
python -m unittest discover -s python -p 'test_*.py' -v
```

The executable will be at `src-tauri/target/debug/koffice-mcp` (Windows: `koffice-mcp.exe`). To launch with an MCP-capable agent, configure a **stdio** MCP server with that executable as its command. The transport is one JSON-RPC 2.0 object per UTF-8 newline; stdout contains only protocol messages.

## Tools

- `koffice_snapshot`: read a full Writer/Sheets/Slides snapshot with revision numbers.
- `koffice_execute`: execute 1–50 revision-checked changes atomically (using the shared Rust `automation::execute` engine).
- `koffice_reset`: discard the isolated session's in-memory documents.

Supported edits: `writer_edit`, `sheets_put_batch`, `sheets_add_sheet`, `sheets_rename_sheet`, `slides_add`, `slides_update`, `slides_duplicate`. Every operation needs the current document's `expected_revision`. When the same document is changed twice in one batch, increment its expected revision accordingly.

Example Python client:

```python
from python.koffice_mcp import KofficeClient
with KofficeClient('src-tauri/target/debug/koffice-mcp') as office:
    office.execute([
        {'type': 'writer_edit', 'expected_revision': 0,
         'edit': {'type': 'replace_all', 'text': 'Koffice AI'}},
        {'type': 'sheets_put_batch', 'expected_revision': 0, 'sheet': 0,
         'updates': {'A1': '5', 'B1': '=A1*3'}}
    ])
    print(office.snapshot())
```

## Security boundaries

No MCP tool can read arbitrary paths, execute processes, save files, or connect to a DB. The server's entire workspace exists only until its process terminates. MCP clients must not assume access to Koffice Desktop, and must not claim successful on-disk saves. Messages over 1 MiB are rejected. For advanced workflows, use only trusted MCP clients and inspect the batch before issuing it.

## Verification

CI now executes Rust tests for the new binary and Python integration tests against the actual compiled executable on Linux. The existing desktop and Windows installer checks remain. See the Actions page for the commit's test results.

**Not implemented yet:** live desktop MCP bridge, file import/export through MCP, Base database MCP tools, DOCX/XLSX/PPTX compatibility, signed installers, user permission policies.

[Repository](https://github.com/konstive-collab/Koffice)
