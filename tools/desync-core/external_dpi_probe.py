#!/usr/bin/env python3
"""Explicit, single-connection Windows test of an UNVERIFIED Zapret 2 TCP profile.

No automatic execution, privilege elevation, proxy changes, DNS changes, or CI trigger.
No external connections in planning mode. Not proof of censorship circumvention.
Only the one test TCP source port and one public IPv4 destination are intercepted.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import http.client
import ipaddress
import pathlib
import re
import shutil
import signal
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time

# Taken from ochistka-core/src/desync.rs::PINNED_WINDOWS (21 November 2026).
PINNED_FILES = {
    "winws2.exe": "5390cb6cec8cdb39bda657f83e8b87ce8dc96d074bf6a71e22c03c6896bcaf42",
    "cygwin1.dll": "ed9d5379061e5de79c3375cc62246c2142ea1c97d94f4d16bfbea6fb1b7fc889",
    "cygz.dll": "02561b4b4b473c40e9bed9c5df59334b5d79efae568b6e212a7c9d775ba8e647",
    "WinDivert.dll": "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2",
    "WinDivert64.sys": "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2",
}
LUA_PINNED = {
    # Unmodified bol-van/zapret2@1eb4fd88 scripts (same files as assets/desync and the Rust adapter).
    "zapret-lib.lua": "79046853613128d8fcba19921a90ec91187f877ead6b893b155c1d7f7804e74e",
    "zapret-antidpi.lua": "ff6165a12d37915a98caef6462fe5bdd0138e567e447d1f65e33b5bb4e398424",
}
LUA_FILES = tuple(LUA_PINNED)
PORT = 443
TIMEOUT_SECONDS = 7.0
STARTUP_SECONDS = 12.0
PROBE_DEADLINE_SECONDS = 24.0
HOST_LABEL = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$", re.ASCII)



def windows_executable(name: str) -> str:
    """Use only trusted Windows System32 executables; never resolve through PATH."""
    paths = {
        "tasklist.exe": ("tasklist.exe",),
        "reg.exe": ("reg.exe",),
        "powershell.exe": ("WindowsPowerShell", "v1.0", "powershell.exe"),
    }
    if name not in paths:
        raise ValueError("Unexpected Windows helper")
    if sys.platform == "win32":
        # SystemRoot is environment-controlled. Query the actual OS directory
        # so an elevated diagnostic never executes binaries from a spoofed path.
        import ctypes

        directory = ctypes.create_unicode_buffer(1024)
        length = ctypes.windll.kernel32.GetSystemDirectoryW(directory, len(directory))
        if length <= 0 or length >= len(directory):
            raise RuntimeError("Cannot locate the trusted Windows system directory")
        root = pathlib.PureWindowsPath(directory.value)
    else:
        # Offline unit tests also run on Linux, never invoking these executables.
        root = pathlib.PureWindowsPath(r"C:\Windows\System32")
    if not root.is_absolute():
        raise RuntimeError("Cannot resolve the trusted Windows system directory")
    return str(root.joinpath(*paths[name]))


def checked_windows_output(name: str, *args: str) -> str:
    """If a preflight check fails, refuse to start the packet driver."""
    result = subprocess.run(
        [windows_executable(name), *args],
        capture_output=True, text=True, timeout=15, check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Windows preflight check failed: {name}")
    return result.stdout


@dataclass(frozen=True)
class ProbeResult:
    outcome: str
    response_status: int | None
    elapsed_ms: int
    lua_callback_confirmed: bool | None = None

    @property
    def reachable(self) -> bool:
        # A redirect proves we contacted this HTTPS origin, but isn't followed.
        return self.response_status is not None and 200 <= self.response_status < 400


def validated_target(host: str, ip: str) -> tuple[str, str]:
    """Refuse URLs, literal IP hosts, private/special addresses and unsafe host syntax."""
    if host != host.strip() or not host or any(c in host for c in "/:@\\?#%"):
        raise ValueError("Host must be a plain DNS name (no URL, credentials, or path)")
    try:
        hostname = host.rstrip(".").encode("idna").decode("ascii").lower()
    except UnicodeError as exc:
        raise ValueError("Invalid DNS hostname") from exc
    labels = hostname.split(".")
    # A literal IP is not a hostname, even though it consists of legal labels.
    try:
        ipaddress.ip_address(hostname)
    except ValueError:
        pass
    else:
        raise ValueError("An IP literal cannot be used as a TLS hostname")
    if len(hostname) > 253 or len(labels) < 2 or not all(HOST_LABEL.fullmatch(x) for x in labels):
        raise ValueError("A fully qualified DNS hostname is required")
    try:
        address = ipaddress.ip_address(ip)
    except ValueError as exc:
        raise ValueError("Invalid IPv4 address") from exc
    if not isinstance(address, ipaddress.IPv4Address) or not safe_public_ipv4(address):
        raise ValueError("Only a public unicast IPv4 test destination is allowed")
    return hostname, str(address)


def safe_public_ipv4(address: ipaddress.IPv4Address) -> bool:
    return address.is_global and not any((
        address.is_multicast,
        address.is_reserved,
        address.is_loopback,
        address.is_link_local,
        address.is_private,
        address.is_unspecified,
    ))


def restricted_filter(ip: str, source_port: int) -> str:
    if not (1024 <= source_port <= 65535):
        raise ValueError("Unsafe client source port")
    # Defense in depth: reject unvalidated raw filter fragments from CLI or files.
    address = ipaddress.IPv4Address(ip)
    if not safe_public_ipv4(address):
        raise ValueError("A public unicast test IPv4 destination is required")
    return (
        f"outbound and ip and tcp and ip.DstAddr == {address} "
        f"and tcp.DstPort == {PORT} and tcp.SrcPort == {source_port}"
    )


def candidate_args(raw_filter: str) -> list[str]:
    """HTTP/TLS candidate in ochistka-core; never used by the app's switch."""
    return [
        f"--wf-raw={raw_filter}",
        "--wf-dup-check=0",
        "--lua-init=@zapret-lib.lua",
        "--lua-init=@zapret-antidpi.lua",
        f"--filter-tcp={PORT}",
        "--filter-l7=tls",
        "--payload=tls_client_hello",
        "--lua-desync=fake:blob=fake_default_tls:tcp_md5:tls_mod=rnd,rndsni,dupsid",
        "--payload=tls_client_hello",
        "--lua-desync=multisplit:pos=1:seqovl=5:seqovl_pattern=0x1603030000",
    ]


def file_sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_release_files(path: pathlib.Path) -> None:
    for filename, expected in PINNED_FILES.items():
        target = path / filename
        if not target.is_file() or target.is_symlink():
            raise RuntimeError(f"Pinned engine file missing or unsafe: {filename}")
        if file_sha256(target) != expected:
            raise RuntimeError(f"Pinned SHA-256 mismatch: {filename}")


def lua_source_dir() -> pathlib.Path:
    """Repository checkout, or the `assets/desync` folder next to a packaged executable."""
    if getattr(sys, "frozen", False):
        return pathlib.Path(sys.executable).resolve().parent / "assets" / "desync"
    return pathlib.Path(__file__).resolve().parents[2] / "assets" / "desync"


def verify_lua_files(folder: pathlib.Path) -> None:
    for name, expected in LUA_PINNED.items():
        target = folder / name
        if not target.is_file() or target.is_symlink():
            raise RuntimeError(f"Missing original Lua: {name}")
        if file_sha256(target) != expected:
            raise RuntimeError(f"Pinned SHA-256 mismatch (Lua): {name}")


def prepare_runtime(
    bundle: pathlib.Path, runtime: pathlib.Path, lua_dir: pathlib.Path | None = None
) -> pathlib.Path:
    verify_release_files(bundle)
    source_dir = lua_dir if lua_dir is not None else lua_source_dir()
    verify_lua_files(source_dir)
    for name in PINNED_FILES:
        shutil.copyfile(bundle / name, runtime / name)
    for name in LUA_FILES:
        shutil.copyfile(source_dir / name, runtime / name)
    verify_release_files(runtime)
    verify_lua_files(runtime)
    return runtime / "winws2.exe"


def check_driver_signature(driver: pathlib.Path) -> None:
    # Do not accept a modified or untrusted Windows kernel driver.
    quoted = str(driver).replace("'", "''")
    command = (
        f"$s = Get-AuthenticodeSignature -LiteralPath '{quoted}'; "
        "if ($s.Status -ne 'Valid') { exit 6 }"
    )
    check = subprocess.run(
        [windows_executable("powershell.exe"), "-NoProfile", "-NonInteractive", "-Command", command],
        capture_output=True,
        timeout=15,
        check=False,
    )
    if check.returncode != 0:
        raise RuntimeError("Pinned WinDivert driver signature is not Valid")


def probe_https(host: str, ip: str, sock: socket.socket | None = None) -> ProbeResult:
    started = time.monotonic()
    # Keep the failing protocol phase, not the server response or packet contents.
    # A TCP-connect timeout is not evidence that the TLS ClientHello strategy ran.
    phase = "tcp_connect"
    try:
        own_socket = sock is None
        if own_socket:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        assert sock is not None
        try:
            sock.settimeout(TIMEOUT_SECONDS)
            sock.connect((ip, PORT))
            phase = "tls_handshake"
            context = ssl.create_default_context()
            # SNI and CA validation refer to the same host supplied by the user.
            with context.wrap_socket(sock, server_hostname=host) as tls:
                tls.settimeout(TIMEOUT_SECONDS)
                phase = "http_request"
                request = (
                    f"GET / HTTP/1.1\r\nHost: {host}\r\n"
                    "User-Agent: OCHISTKA-external-probe/1.0\r\n"
                    "Accept: */*\r\nConnection: close\r\n\r\n"
                )
                tls.sendall(request.encode("ascii"))
                phase = "http_response"
                response = http.client.HTTPResponse(tls)
                response.begin()
                status = response.status
                response.read(256)  # Minimal response; no cookies, content logging, or redirects.
                response.close()
                outcome = "http_success" if 200 <= status < 400 else "http_non_success"
                return ProbeResult(outcome, status, round((time.monotonic() - started) * 1000))
        finally:
            # Closing an already TLS-wrapped socket is harmless; no sockets escape.
            sock.close()
    except socket.timeout:
        outcome = f"timeout_{phase}"
    except ssl.SSLCertVerificationError:
        outcome = "certificate_validation_failed"
    except ssl.SSLError:
        outcome = "tls_failed"
    except (ConnectionError, OSError):
        outcome = f"connection_failed_{phase}"
    except http.client.HTTPException:
        outcome = "http_invalid"
    return ProbeResult(outcome, None, round((time.monotonic() - started) * 1000))



def bounded_probe_https(host: str, ip: str, sock: socket.socket | None = None) -> ProbeResult:
    """Limit the whole probe, including servers trickling endless HTTP headers."""
    client = sock if sock is not None else socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    result: list[ProbeResult] = []
    error: list[BaseException] = []
    started = time.monotonic()

    def run() -> None:
        try:
            result.append(probe_https(host, ip, client))
        except BaseException as exc:
            error.append(exc)

    worker = threading.Thread(target=run, name="ochistka-single-probe", daemon=True)
    worker.start()
    worker.join(timeout=PROBE_DEADLINE_SECONDS)
    if worker.is_alive():
        # Closing a socket may not interrupt an already wrapped SSLSocket.
        # Never continue to the engine or the following control while a probe is alive.
        try:
            client.shutdown(socket.SHUT_RDWR)
        except (OSError, AttributeError):
            pass
        client.close()
        worker.join(timeout=0.5)
        if worker.is_alive():
            raise RuntimeError("HTTPS probe deadline exceeded and worker has not stopped")
        return ProbeResult("probe_deadline", None, round((time.monotonic() - started) * 1000))
    if error:
        raise RuntimeError("Unexpected error in bounded HTTPS probe") from error[0]
    return result[0]


def wait_capture(proc: subprocess.Popen[bytes], log: pathlib.Path) -> None:
    deadline = time.monotonic() + STARTUP_SECONDS
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise RuntimeError("Winws2 ended before the narrow test capture was ready")
        try:
            if "windivert initialized. capture is started." in log.read_text(
                encoding="utf-8", errors="replace"
            )[-10000:]:
                return
        except OSError:
            pass
        time.sleep(0.15)
    raise RuntimeError("Timed out waiting for narrow WinDivert capture")


def stop_engine(proc: subprocess.Popen[bytes] | None) -> bool:
    """Stop our own engine process. Returns True only when it is certainly not running."""
    if proc is None or proc.poll() is not None:
        return True
    for stop in (lambda: proc.send_signal(signal.CTRL_BREAK_EVENT), proc.kill):
        try:
            stop()
            proc.wait(timeout=3)
            return True
        except (OSError, subprocess.TimeoutExpired, AttributeError):
            continue
    return proc.poll() is not None


def engine_log_has_live_callback(log: pathlib.Path) -> bool:
    """Require live packet interception AND a real upstream Lua callback."""
    try:
        with log.open("rb") as stream:
            stream.seek(0, 2)
            stream.seek(max(0, stream.tell() - 262144))
            data = stream.read().decode("utf-8", errors="replace")
    except OSError:
        return False
    return "packet: id=" in data and (
        "LUA: fake:" in data or "LUA: multisplit:" in data
    )


def wait_engine_evidence(log: pathlib.Path, proc: subprocess.Popen[bytes]) -> bool:
    deadline = time.monotonic() + 3.0
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return False
        if engine_log_has_live_callback(log):
            return True
        time.sleep(0.1)
    return False


def active_probe(exe: pathlib.Path, host: str, ip: str, log: pathlib.Path) -> ProbeResult:
    # Reserve an ephemeral source port BEFORE launching WinDivert. This lets
    # the filter capture only our own outbound test connection (not the browser).
    client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    proc: subprocess.Popen[bytes] | None = None
    try:
        client.bind(("0.0.0.0", 0))
        source_port = client.getsockname()[1]
        args = candidate_args(restricted_filter(ip, source_port))
        # Validate arguments without starting WinDivert first.
        dry = subprocess.run(
            [str(exe), "--dry-run", *args], cwd=str(exe.parent),
            capture_output=True, timeout=10, check=False,
        )
        if dry.returncode != 0:
            raise RuntimeError("Pinned engine rejected the candidate arguments")
        with log.open("wb") as output:
            proc = subprocess.Popen(
                [str(exe), "--debug=1", *args], cwd=str(exe.parent),
                stdout=output, stderr=subprocess.STDOUT,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
            )
            wait_capture(proc, log)
            # One TLS request and one target only, while the filter is active.
            result = bounded_probe_https(host, ip, client)
            if proc.poll() is not None:
                raise RuntimeError("Pinned engine stopped unexpectedly during active probe")
            callback = engine_log_has_live_callback(log)
            if result.reachable and not callback:
                callback = wait_engine_evidence(log, proc)
                if not callback:
                    raise RuntimeError(
                        "HTTPS connected, but real packet interception and Lua callback were not confirmed"
                    )
            return ProbeResult(
                result.outcome, result.response_status, result.elapsed_ms,
                lua_callback_confirmed=callback,
            )
    finally:
        # Stop FIRST; Windows then releases the temporary packet filter.
        stopped = stop_engine(proc)
        client.close()
        if not stopped:
            raise RuntimeError("Could not stop the test engine process; stop it manually (Task Manager)")


def interpret(before: ProbeResult, active: ProbeResult, after: ProbeResult) -> str:
    """Connectivity evidence only. NONE of these labels establishes DPI causality."""
    if before.reachable:
        return "inconclusive_baseline_already_reachable"
    if not active.reachable:
        return "no_confirmed_improvement"
    if after.reachable or before.outcome != after.outcome:
        return "inconclusive_control_changed"
    return "repeatable_connectivity_improvement_not_proof_of_dpi"


def other_engine_running() -> bool:
    """Detection only: fail closed if the process inventory cannot be read."""
    out = checked_windows_output(
        "tasklist.exe", "/FI", "IMAGENAME eq winws2.exe", "/FO", "CSV", "/NH",
    )
    return bool(re.search(r'(?im)^"winws2\.exe",', out))


def system_proxy_enabled() -> bool:
    """Read-only check. Deny explicit proxy and PAC, fail closed on an unreadable registry."""
    out = checked_windows_output(
        "reg.exe", "query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
    )
    switch = re.search(r"(?im)^\s*ProxyEnable\s+REG_DWORD\s+0x([0-9a-f]+)\s*$", out)
    if switch is None:
        raise RuntimeError("Cannot verify system proxy state; test is blocked")
    pac = re.search(r"(?im)^\s*AutoConfigURL\s+REG_\w+\s+\S+", out)
    return int(switch.group(1), 16) != 0 or pac is not None


def is_admin_windows() -> bool:
    import ctypes

    return bool(ctypes.windll.shell32.IsUserAnAdmin())


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True, help="Only DNS host, no path, port or URL")
    parser.add_argument("--ip", required=True, help="Public IPv4 endpoint for that hostname")
    parser.add_argument("--bundle-dir", type=pathlib.Path, help="Directory extracted from the pinned Release")
    parser.add_argument("--execute", action="store_true", help="Actually run the gated Windows test")
    parser.add_argument("--acknowledge-narrow-interception", action="store_true")
    parser.add_argument("--confirm-vpn-off", action="store_true")
    options = parser.parse_args(argv)
    host, ip = validated_target(options.host, options.ip)
    if not options.execute:
        print("PLAN ONLY: no network connections, no driver, no files changed")
        print(f"Target: https://{host}/ via {ip}:{PORT} (TLS certificate validated)")
        print("A real run requires --execute, both explicit acknowledgements, and a pinned local bundle.")
        return 0
    if not (options.acknowledge_narrow_interception and options.confirm_vpn_off):
        parser.error("Execution requires explicit interception and disconnected-VPN acknowledgements")
    if options.bundle_dir is None:
        parser.error("Execution requires --bundle-dir with the pinned Release files")
    if sys.platform != "win32" or not is_admin_windows():
        raise RuntimeError("Only an explicitly elevated Windows session can run this test")
    if options.bundle_dir.resolve() == pathlib.Path.cwd().resolve():
        raise RuntimeError("Do not use the current repository as an engine bundle directory")
    if system_proxy_enabled():
        raise RuntimeError("A system proxy is enabled (VPN in proxy mode?): disconnect it first")
    if other_engine_running():
        raise RuntimeError("Another winws2.exe is already running; stop it first (it is never killed here)")

    # The initial control must show a real problem, or the test is meaningless.
    with tempfile.TemporaryDirectory(prefix="ochistka-external-probe-") as folder:
        runtime = pathlib.Path(folder)
        exe = prepare_runtime(options.bundle_dir.resolve(), runtime)
        check_driver_signature(runtime / "WinDivert64.sys")
        before = bounded_probe_https(host, ip)
        print(f"Before: {before.outcome} (HTTP {before.response_status}, {before.elapsed_ms} ms)", flush=True)
        if before.reachable:
            print("INCONCLUSIVE: baseline already reachable; engine not started.", flush=True)
            return 2
        # No external network operations except the test host/IP connection.
        active = active_probe(exe, host, ip, runtime / "engine.log")
        print(f"With candidate: {active.outcome} (HTTP {active.response_status}, {active.elapsed_ms} ms)", flush=True)
        print(
            f"Lua callback confirmed during candidate: {'yes' if active.lua_callback_confirmed else 'no'}"
            " (no does not establish absence of capture)",
            flush=True,
        )
        after = bounded_probe_https(host, ip)
        print(f"After shutdown: {after.outcome} (HTTP {after.response_status}, {after.elapsed_ms} ms)", flush=True)
        verdict = interpret(before, active, after)
        if before.outcome == "timeout_tcp_connect" and active.outcome == "timeout_tcp_connect":
            print(
                "DIAGNOSIS: TCP connection did not complete; the TLS ClientHello"
                " strategy could not act on a completed TLS handshake.",
                flush=True,
            )
        print(f"VERDICT: {verdict}", flush=True)
        print("These observations do not prove the cause was DPI or establish a shippable profile.")
        return 0 if verdict == "repeatable_connectivity_improvement_not_proof_of_dpi" else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, ValueError, OSError, subprocess.TimeoutExpired) as exc:
        print(f"ABORTED: {exc}", file=sys.stderr, flush=True)
        raise SystemExit(1)