"""Koffice MCP subprocess client (Python 3.10+, no third-party dependencies).

The agent owns an isolated, in-memory Koffice document session. This does not
connect to live desktop tabs and never grants filesystem access to a tool.
"""
from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any


class KofficeError(RuntimeError):
    pass


class KofficeClient:
    def __init__(self, executable: str | os.PathLike[str] = "koffice-mcp") -> None:
        self.executable = str(executable)
        self._process: subprocess.Popen[str] | None = None
        self._seq = 0

    def __enter__(self) -> "KofficeClient":
        self._process = subprocess.Popen(
            [self.executable], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, encoding="utf-8", bufsize=1,
        )
        self.request("initialize", {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "koffice-python", "version": "0.8.0"},
        })
        self.notify("notifications/initialized")
        return self

    def __exit__(self, *_: object) -> None:
        if self._process is not None:
            self._process.terminate()
            try:
                self._process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait()
            finally:
                self._process = None

    def _io(self) -> tuple[Any, Any]:
        if self._process is None or self._process.poll() is not None:
            raise KofficeError("Koffice MCP server is not running")
        assert self._process.stdin is not None
        assert self._process.stdout is not None
        return self._process.stdin, self._process.stdout

    def notify(self, method: str, params: dict[str, Any] | None = None) -> None:
        stdin, _ = self._io()
        stdin.write(json.dumps({"jsonrpc": "2.0", "method": method, "params": params or {}}, ensure_ascii=False) + "\n")
        stdin.flush()

    def request(self, method: str, params: dict[str, Any] | None = None) -> Any:
        stdin, stdout = self._io()
        self._seq += 1
        payload = {"jsonrpc": "2.0", "id": self._seq, "method": method, "params": params or {}}
        stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        stdin.flush()
        line = stdout.readline()
        if not line:
            raise KofficeError("MCP server closed the connection")
        reply = json.loads(line)
        if reply.get("id") != self._seq:
            raise KofficeError("Mismatched JSON-RPC response ID")
        if "error" in reply:
            raise KofficeError(str(reply["error"]))
        return reply["result"]

    def tools(self) -> list[dict[str, Any]]:
        return self.request("tools/list")["tools"]

    def call(self, name: str, arguments: dict[str, Any] | None = None) -> Any:
        result = self.request("tools/call", {"name": name, "arguments": arguments or {}})
        if result.get("isError"):
            raise KofficeError(result["content"][0]["text"])
        return json.loads(result["content"][0]["text"])

    def snapshot(self) -> dict[str, Any]:
        return self.call("koffice_snapshot")

    def execute(self, operations: list[dict[str, Any]]) -> dict[str, Any]:
        return self.call("koffice_execute", {"operations": operations})

    def reset(self) -> dict[str, Any]:
        return self.call("koffice_reset")


def example() -> None:
    with KofficeClient() as office:
        office.execute([
            {"type": "writer_edit", "expected_revision": 0,
             "edit": {"type": "replace_all", "text": "Hello from Python!"}},
            {"type": "sheets_put_batch", "expected_revision": 0,
             "sheet": 0, "updates": {"A1": "42", "B1": "=A1*2"}},
            {"type": "slides_add", "expected_revision": 0},
        ])
        print(json.dumps(office.snapshot(), indent=2, ensure_ascii=False))


if __name__ == "__main__":
    example()
