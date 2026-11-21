"""Офлайн-проверки простого запуска: без сети, драйвера и прав администратора."""

from __future__ import annotations

import importlib.util
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("ochistka_dpi_check", HERE / "ochistka_dpi_check.py")
assert spec is not None and spec.loader is not None
launcher = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = launcher
spec.loader.exec_module(launcher)
probe = launcher.probe


def fake_bundle(folder: pathlib.Path) -> None:
    for name in probe.PINNED_FILES:
        (folder / name).write_bytes(b"x")


class SelfCheckTests(unittest.TestCase):
    def test_wrong_files_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            fake_bundle(pathlib.Path(folder))
            with self.assertRaises(RuntimeError):
                launcher.self_check(pathlib.Path(folder), probe.lua_source_dir())

    def test_missing_bundle_fails(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(RuntimeError):
                launcher.self_check(pathlib.Path(folder))

    def test_checked_in_lua_matches_pins_and_tampering_is_rejected(self) -> None:
        probe.verify_lua_files(probe.lua_source_dir())
        with tempfile.TemporaryDirectory() as folder:
            target = pathlib.Path(folder)
            for name in probe.LUA_FILES:
                (target / name).write_bytes((probe.lua_source_dir() / name).read_bytes())
            probe.verify_lua_files(target)
            (target / probe.LUA_FILES[0]).write_bytes(b"-- changed")
            with self.assertRaises(RuntimeError):
                probe.verify_lua_files(target)

    def test_self_check_mode_uses_no_network_or_processes(self) -> None:
        with mock.patch.object(launcher.socket, "socket") as sock, \
             mock.patch.object(probe.subprocess, "run") as run, \
             mock.patch.object(probe.subprocess, "Popen") as popen:
            with tempfile.TemporaryDirectory() as folder, \
                 mock.patch.object(launcher, "bundle_dir", return_value=pathlib.Path(folder)):
                self.assertEqual(launcher.main([launcher.SELF_CHECK]), 1)
            sock.assert_not_called(); run.assert_not_called(); popen.assert_not_called()


class ConsentAndTargetTests(unittest.TestCase):
    def test_only_the_word_yes_counts(self) -> None:
        for answer, expected in [("ДА", True), ("да", True), (" Да ", True), ("", False), ("yes", False), ("y", False), ("нет", False)]:
            self.assertEqual(launcher.ask_yes("?", lambda _q, a=answer: a), expected, answer)

    def test_private_and_special_addresses_from_dns_are_dropped(self) -> None:
        answers = [(2, 1, 6, "", (ip, 443)) for ip in ["127.0.0.1", "10.0.0.5", "192.168.1.1", "93.184.216.34", "93.184.216.34"]]
        with mock.patch.object(launcher.socket, "getaddrinfo", return_value=answers):
            self.assertEqual(launcher.resolve_ipv4("example.org"), ["93.184.216.34"])

    def test_unsafe_host_names_are_refused_before_any_lookup(self) -> None:
        with mock.patch.object(launcher.socket, "getaddrinfo") as lookup:
            for host in ["https://example.org", "127.0.0.1", "a@b.org", "example.org/x"]:
                with self.assertRaises(ValueError):
                    launcher.resolve_ipv4(host)
            lookup.assert_not_called()

    def test_refusal_never_starts_the_engine(self) -> None:
        answers = iter(["example.org", ""])  # хост, затем отказ на первом согласии
        with mock.patch.object(launcher.sys, "platform", "win32"), \
             mock.patch.object(probe, "is_admin_windows", return_value=True), \
             mock.patch.object(launcher, "self_check"), \
             mock.patch.object(launcher, "resolve_ipv4", return_value=["93.184.216.34"]), \
             mock.patch.object(probe, "main") as run:
            self.assertEqual(launcher.run_interactive(lambda _q: next(answers)), 0)
            run.assert_not_called()

    def test_vpn_not_confirmed_never_starts_the_engine(self) -> None:
        answers = iter(["example.org", "да", "нет"])
        with mock.patch.object(launcher.sys, "platform", "win32"), \
             mock.patch.object(probe, "is_admin_windows", return_value=True), \
             mock.patch.object(launcher, "self_check"), \
             mock.patch.object(launcher, "resolve_ipv4", return_value=["93.184.216.34"]), \
             mock.patch.object(probe, "main") as run:
            self.assertEqual(launcher.run_interactive(lambda _q: next(answers)), 0)
            run.assert_not_called()

    def test_without_administrator_rights_nothing_runs_and_no_elevation_is_attempted(self) -> None:
        with mock.patch.object(launcher.sys, "platform", "win32"), \
             mock.patch.object(probe, "is_admin_windows", return_value=False), \
             mock.patch.object(probe.subprocess, "run") as run, \
             mock.patch.object(probe, "main") as engine:
            self.assertEqual(launcher.run_interactive(lambda _q: "x"), 1)
            run.assert_not_called(); engine.assert_not_called()

    def test_failed_hash_check_stops_before_any_question(self) -> None:
        ask = mock.Mock()
        with mock.patch.object(launcher.sys, "platform", "win32"), \
             mock.patch.object(probe, "is_admin_windows", return_value=True), \
             mock.patch.object(launcher, "self_check", side_effect=RuntimeError("mismatch")):
            self.assertEqual(launcher.run_interactive(ask), 1)
            ask.assert_not_called()

    def test_full_consent_passes_only_the_expected_flags_and_writes_a_safe_report(self) -> None:
        answers = iter(["Example.ORG", "да", "да"])
        with tempfile.TemporaryDirectory() as folder, \
             mock.patch.object(launcher.sys, "platform", "win32"), \
             mock.patch.object(probe, "is_admin_windows", return_value=True), \
             mock.patch.object(launcher, "self_check"), \
             mock.patch.object(launcher, "app_dir", return_value=pathlib.Path(folder)), \
             mock.patch.object(launcher, "resolve_ipv4", return_value=["93.184.216.34"]), \
             mock.patch.object(probe, "main", return_value=2) as run:
            code = launcher.run_interactive(lambda _q: next(answers), pathlib.Path(folder) / "bundle")
            self.assertEqual(code, 2)
            argv = run.call_args.args[0]
            self.assertEqual(argv[:4], ["--host", "example.org", "--ip", "93.184.216.34"])
            for flag in ["--execute", "--acknowledge-narrow-interception", "--confirm-vpn-off"]:
                self.assertIn(flag, argv)
            reports = list(pathlib.Path(folder).glob("OCHISTKA-DPI-report-*.txt"))
            self.assertEqual(len(reports), 1)
            self.assertIn("example.org", reports[0].read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
