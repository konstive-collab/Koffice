"""Integration tests for the real Rust MCP binary."""
import os
import unittest

from koffice_mcp import KofficeClient, KofficeError

BIN = os.environ.get("KOFFICE_MCP_BIN", "src-tauri/target/debug/koffice-mcp")


class MCPIntegrationTests(unittest.TestCase):
    def test_discovery_and_reset(self):
        with KofficeClient(BIN) as office:
            names = {t["name"] for t in office.tools()}
            self.assertEqual(names, {"koffice_snapshot", "koffice_execute", "koffice_reset"})
            office.execute([{"type": "slides_add", "expected_revision": 0}])
            self.assertEqual(len(office.snapshot()["slides"]["slides"]), 2)
            self.assertEqual(len(office.reset()["slides"]["slides"]), 1)

    def test_cross_editor_transaction_and_rollback(self):
        with KofficeClient(BIN) as office:
            result = office.execute([
                {"type": "writer_edit", "expected_revision": 0,
                 "edit": {"type": "replace_all", "text": "Привет ☕"}},
                {"type": "sheets_put_batch", "expected_revision": 0,
                 "sheet": 0, "updates": {"A1": "10"}},
            ])
            self.assertEqual(result["result"]["applied"], 2)
            self.assertEqual(office.snapshot()["writer"]["text"], "Привет ☕")
            with self.assertRaises(KofficeError):
                office.execute([
                    {"type": "writer_edit", "expected_revision": 1,
                     "edit": {"type": "replace_all", "text": "bad"}},
                    {"type": "sheets_put_batch", "expected_revision": 1,
                     "sheet": 0, "updates": {"A0": "invalid"}},
                ])
            self.assertEqual(office.snapshot()["writer"]["text"], "Привет ☕")
            self.assertEqual(office.snapshot()["sheets"]["sheets"][0]["cells"]["A1"], "10")

    def test_rejects_unknown_tools(self):
        with KofficeClient(BIN) as office:
            with self.assertRaises(KofficeError):
                office.call("execute_shell", {"command": "pwd"})


if __name__ == "__main__":
    unittest.main()
