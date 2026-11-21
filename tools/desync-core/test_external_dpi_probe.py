"""Offline-only checks: never start WinDivert or make outbound requests."""

from __future__ import annotations

import importlib.util
import pathlib
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location(
    "external_dpi_probe", pathlib.Path(__file__).with_name("external_dpi_probe.py")
)
assert spec is not None and spec.loader is not None
import sys
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)


class SafePlanTests(unittest.TestCase):
    def test_public_dns_target_and_ip(self) -> None:
        self.assertEqual(
            module.validated_target("ExAmPlE.Org", "1.1.1.1"), ("example.org", "1.1.1.1")
        )

    def test_domain_normalizes_unicode_to_ascii_idna(self) -> None:
        self.assertEqual(module.validated_target("bücher.example", "1.1.1.1")[0], "xn--bcher-kva.example")

    def test_reject_unsafe_hosts(self) -> None:
        for host in ["https://example.com", "user@example.com", "example.com/path", "example.com:443", "localhost", "*.com", "a..b", "test.com?secret=x", " example.org", "127.0.0.1"]:
            with self.subTest(host=host), self.assertRaises(ValueError):
                module.validated_target(host, "1.1.1.1")

    def test_reject_special_ips(self) -> None:
        for ip in ["127.0.0.1", "192.168.1.3", "10.1.2.3", "169.254.3.1", "224.0.0.1", "255.255.255.255", "2001:4860:4860::8888", "203.0.113.1", "some-hostname", "1.1.1.1,127.0.0.1"]:
            with self.subTest(ip=ip), self.assertRaises(ValueError):
                module.validated_target("example.org", ip)

    def test_filter_exact_destination_and_ephemeral_client_port(self) -> None:
        result = module.restricted_filter("1.1.1.1", 54321)
        self.assertEqual(result, "outbound and ip and tcp and ip.DstAddr == 1.1.1.1 and tcp.DstPort == 443 and tcp.SrcPort == 54321")
        with self.assertRaises(ValueError):
            module.restricted_filter("127.0.0.1", 54321)
        with self.assertRaises(ValueError):
            module.restricted_filter("1.1.1.1", 22)

    def test_candidate_is_tightly_scoped(self) -> None:
        args = module.candidate_args(module.restricted_filter("1.1.1.1", 54321))
        self.assertEqual(args[0].count("--wf-raw="), 1)
        self.assertIn("--filter-tcp=443", args)
        self.assertIn("--filter-l7=tls", args)
        self.assertNotIn("--wf-tcp-out=80,443", args)
        self.assertEqual(len([x for x in args if x.startswith("--lua-init=")]), 2)

    def test_planning_never_opens_socket_or_subprocess(self) -> None:
        with mock.patch.object(module.socket, "socket") as create_socket, mock.patch.object(module.subprocess, "run") as run:
            self.assertEqual(module.main(["--host", "example.org", "--ip", "1.1.1.1"]), 0)
            create_socket.assert_not_called()
            run.assert_not_called()

    def test_execution_requires_two_flags(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            module.main(["--host", "example.org", "--ip", "1.1.1.1", "--execute"])
        self.assertEqual(raised.exception.code, 2)

    def test_checked_in_core_pin_matches_python_tool(self) -> None:
        core = pathlib.Path(__file__).resolve().parents[2] / "crates" / "ochistka-core" / "src" / "desync.rs"
        if not core.is_file():
            self.skipTest("Full repository checkout required for pin sync check")
        source = core.read_text(encoding="utf-8")
        for filename, digest in module.PINNED_FILES.items():
            self.assertIn(f'("{filename}", "{digest}")', source)

    def test_bounded_probe_cancels_when_deadline_expires(self) -> None:
        client = mock.Mock()
        with mock.patch.object(module, "PROBE_DEADLINE_SECONDS", 0.001), mock.patch.object(module, "probe_https", side_effect=lambda *args: __import__("time").sleep(0.1)):
            result = module.bounded_probe_https("example.org", "1.1.1.1", client)
        self.assertEqual(result.outcome, "probe_deadline")
        client.close.assert_called_once()

    def test_invalid_files_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaisesRegex(RuntimeError, "missing"):
                module.verify_release_files(pathlib.Path(temp))
            (pathlib.Path(temp) / "winws2.exe").write_bytes(b"wrong bytes")
            with self.assertRaisesRegex(RuntimeError, "mismatch"):
                module.verify_release_files(pathlib.Path(temp))


class EvidenceTests(unittest.TestCase):
    @staticmethod
    def result(outcome: str, status: int | None = None) -> module.ProbeResult:
        return module.ProbeResult(outcome, status, 15)

    def test_success_requires_failed_controls_both_sides(self) -> None:
        verdict = module.interpret(self.result("timeout"), self.result("http_success", 200), self.result("timeout"))
        self.assertEqual(verdict, "repeatable_connectivity_improvement_not_proof_of_dpi")

    def test_baseline_already_up_is_inconclusive(self) -> None:
        self.assertEqual(module.interpret(self.result("http_success", 301), self.result("timeout"), self.result("timeout")), "inconclusive_baseline_already_reachable")

    def test_no_improvement(self) -> None:
        self.assertEqual(module.interpret(self.result("timeout"), self.result("tls_failed"), self.result("timeout")), "no_confirmed_improvement")

    def test_control_change_invalidates_evidence(self) -> None:
        self.assertEqual(module.interpret(self.result("timeout"), self.result("http_success", 200), self.result("http_success", 200)), "inconclusive_control_changed")
        self.assertEqual(module.interpret(self.result("timeout"), self.result("http_success", 200), self.result("connection_failed")), "inconclusive_control_changed")

    def test_http_403_is_not_success(self) -> None:
        self.assertFalse(self.result("http_non_success", 403).reachable)

    def test_redirect_is_reachable_without_following_it(self) -> None:
        self.assertTrue(self.result("http_success", 302).reachable)



class SafetyTests(unittest.TestCase):
    def test_stop_engine_reports_failure_when_process_survives(self) -> None:
        proc = mock.Mock()
        proc.poll.return_value = None
        proc.wait.side_effect = module.subprocess.TimeoutExpired("x", 3)
        with mock.patch.object(module.signal, "CTRL_BREAK_EVENT", 1, create=True):
            self.assertFalse(module.stop_engine(proc))
        proc.kill.assert_called_once()

    def test_stop_engine_true_for_already_finished_process(self) -> None:
        proc = mock.Mock()
        proc.poll.return_value = 0
        self.assertTrue(module.stop_engine(proc))
        proc.send_signal.assert_not_called()

    def test_active_probe_always_stops_engine_even_when_probe_fails(self) -> None:
        proc = mock.Mock()
        proc.poll.return_value = None
        dry = mock.Mock(returncode=0)
        with tempfile.TemporaryDirectory() as folder:
            log = pathlib.Path(folder) / "engine.log"
            exe = pathlib.Path(folder) / "winws2.exe"
            with mock.patch.object(module.subprocess, "run", return_value=dry), \
                 mock.patch.object(module.subprocess, "Popen", return_value=proc), \
                 mock.patch.object(module.subprocess, "CREATE_NEW_PROCESS_GROUP", 0x200, create=True), \
                 mock.patch.object(module, "wait_capture"), \
                 mock.patch.object(module, "bounded_probe_https", side_effect=RuntimeError("boom")), \
                 mock.patch.object(module, "stop_engine", return_value=True) as stop:
                with self.assertRaises(RuntimeError):
                    module.active_probe(exe, "example.org", "1.1.1.1", log)
                stop.assert_called_once_with(proc)

    def test_active_probe_fails_loudly_if_engine_cannot_be_stopped(self) -> None:
        proc = mock.Mock()
        proc.poll.return_value = None
        with tempfile.TemporaryDirectory() as folder:
            log = pathlib.Path(folder) / "engine.log"
            exe = pathlib.Path(folder) / "winws2.exe"
            with mock.patch.object(module.subprocess, "run", return_value=mock.Mock(returncode=0)), \
                 mock.patch.object(module.subprocess, "Popen", return_value=proc), \
                 mock.patch.object(module.subprocess, "CREATE_NEW_PROCESS_GROUP", 0x200, create=True), \
                 mock.patch.object(module, "wait_capture"), \
                 mock.patch.object(module, "bounded_probe_https", return_value=module.ProbeResult("timeout", None, 1)), \
                 mock.patch.object(module, "stop_engine", return_value=False):
                with self.assertRaisesRegex(RuntimeError, "Could not stop"):
                    module.active_probe(exe, "example.org", "1.1.1.1", log)

    def test_rejected_candidate_arguments_never_start_the_driver(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            exe = pathlib.Path(folder) / "winws2.exe"
            with mock.patch.object(module.subprocess, "run", return_value=mock.Mock(returncode=1)), \
                 mock.patch.object(module.subprocess, "Popen") as popen:
                with self.assertRaises(RuntimeError):
                    module.active_probe(exe, "example.org", "1.1.1.1", pathlib.Path(folder) / "l")
                popen.assert_not_called()

    def test_preflight_helpers_detect_proxy_and_other_engine_without_killing(self) -> None:
        def run(stdout: str):
            return mock.Mock(stdout=stdout)
        with mock.patch.object(module.subprocess, "run", return_value=run("    ProxyEnable    REG_DWORD    0x1\n")):
            self.assertTrue(module.system_proxy_enabled())
        with mock.patch.object(module.subprocess, "run", return_value=run("    ProxyEnable    REG_DWORD    0x0\n")):
            self.assertFalse(module.system_proxy_enabled())
        with mock.patch.object(module.subprocess, "run", return_value=run('"winws2.exe","123"\n')) as call:
            self.assertTrue(module.other_engine_running())
            self.assertEqual(call.call_args.args[0][0], "tasklist.exe")
        with mock.patch.object(module.subprocess, "run", return_value=run("INFO: No tasks\n")):
            self.assertFalse(module.other_engine_running())


if __name__ == "__main__":
    unittest.main()
