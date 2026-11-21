#!/usr/bin/env python3
"""Opt-in CI smoke: isolate WinDivert interception to one local HTTP socket.

Not a DPI-circumvention effectiveness test. No requests leave 127.0.0.1.
Uses only stdlib and the already verified winws2.exe Release files.
"""

from __future__ import annotations

import http.client
import http.server
import pathlib
import re
import signal
import subprocess
import sys
import tempfile
import threading
import time

RESPONSE_BODY = b"ochistka-loopback-smoke-ok"
TIMEOUT = 15.0


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:
        self.send_response(200)
        self.send_header("Content-Length", str(len(RESPONSE_BODY)))
        self.send_header("Content-Type", "text/plain")
        self.end_headers()
        self.wfile.write(RESPONSE_BODY)

    def log_message(self, fmt: str, *args: object) -> None:
        return


def inspect_log(log: pathlib.Path, limit: int = 6000) -> str:
    try:
        data = log.read_bytes()
        return data.decode("utf-8", errors="replace")[-limit:]
    except OSError:
        return "<log missing>"


def wait_ready(proc: subprocess.Popen[bytes], log: pathlib.Path) -> None:
    deadline = time.monotonic() + TIMEOUT
    while time.monotonic() < deadline:
        content = inspect_log(log, limit=10000)
        if "windivert initialized. capture is started." in content:
            return
        if proc.poll() is not None:
            raise RuntimeError(
                "winws2 exited before capture initialization "
                f"(exit={proc.returncode}):\n{content}"
            )
        time.sleep(0.2)
    raise RuntimeError("Timed out waiting for WinDivert capture:\n" + inspect_log(log))


def request_local_http(port: int) -> None:
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5.0)
    try:
        conn.request("GET", "/ochistka-test", headers={"Host": "ochistka.local"})
        response = conn.getresponse()
        payload = response.read()
        if response.status != 200 or payload != RESPONSE_BODY:
            raise RuntimeError(f"Unexpected local response: status={response.status}")
    finally:
        conn.close()


def main() -> int:
    if sys.platform != "win32":
        raise RuntimeError("Only run this test on an isolated Windows runner")
    if len(sys.argv) != 2:
        raise RuntimeError("Usage: loopback_smoke.py VERIFIED_RELEASE_DIRECTORY")

    exe = pathlib.Path(sys.argv[1]).resolve() / "winws2.exe"
    if not exe.is_file():
        raise RuntimeError(f"Missing signed-release test executable: {exe}")

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    # The OS chooses a free port. No external listeners or outbound connections.
    port = server.server_port
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    print(f"Test HTTP server bound to 127.0.0.1:{port}", flush=True)

    # WinDivert loopback packets are outbound. This filter matches precisely
    # our localhost server's TCP destination port, no other host/port/traffic.
    isolated_filter = (
        f"outbound and loopback and ip and tcp and "
        f"ip.DstAddr == 127.0.0.1 and tcp.DstPort == {port}"
    )
    with tempfile.TemporaryDirectory(prefix="ochistka-loopback-") as tmp:
        log = pathlib.Path(tmp) / "winws2.log"
        cmd = [
            str(exe),
            "--debug=1",
            f"--wf-raw={isolated_filter}",
            "--wf-dup-check=0",
        ]
        proc: subprocess.Popen[bytes] | None = None
        try:
            with log.open("wb", buffering=0) as output:
                proc = subprocess.Popen(
                    cmd,
                    cwd=str(exe.parent),
                    stdout=output,
                    stderr=subprocess.STDOUT,
                    creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
                )
                wait_ready(proc, log)

                # Request through actual Windows TCP/IP loopback and verify
                # the connection remains functional while WinDivert reinjects.
                for _ in range(16):
                    request_local_http(port)
                    time.sleep(0.05)

                # We require evidence of packets reaching the engine,
                # not merely a successful HTTP request without interception.
                deadline = time.monotonic() + 5.0
                while time.monotonic() < deadline:
                    output_text = inspect_log(log, limit=100000)
                    if "packet: id=" in output_text and "reinject unmodified" in output_text:
                        print("PASS: isolated loopback HTTP succeeds AND packet capture/reinjection logged.", flush=True)
                        return 0
                    if proc.poll() is not None:
                        break
                    time.sleep(0.2)
                raise RuntimeError(
                    "Local HTTP worked but capture/reinjection was not verified. "
                    "This is NOT a successful network test.\n" + inspect_log(log)
                )
        finally:
            if proc is not None and proc.poll() is None:
                try:
                    proc.send_signal(signal.CTRL_BREAK_EVENT)
                    proc.wait(timeout=4.0)
                except (OSError, subprocess.TimeoutExpired):
                    proc.kill()
                    proc.wait(timeout=4.0)
            server.shutdown()
            server.server_close()
            print("Local test server and winws2 stopped; no user settings changed.", flush=True)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"FAIL: {type(exc).__name__}: {exc}", file=sys.stderr, flush=True)
        raise SystemExit(1)
