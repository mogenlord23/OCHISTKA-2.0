#!/usr/bin/env python3
"""Synthetic HTTP test of Zapret 2 Lua desync on an isolated loopback port.

Uses the upstream HTTP branch of OCHISTKA's experimental TCP candidate.
Not a test of ISP censorship, Discord UDP, Telegram, QUIC or TLS.
"""

from __future__ import annotations

import http.server
import pathlib
import signal
import subprocess
import sys
import tempfile
import threading
import time

from loopback_smoke import Handler, inspect_log, request_local_http, wait_ready


def main() -> int:
    if sys.platform != "win32":
        raise RuntimeError("Only permitted on a disposable Windows CI runner")
    if len(sys.argv) != 3 or sys.argv[2] not in ("noop", "fake", "full"):
        raise RuntimeError("Usage: loopback_candidate.py VERIFIED_RELEASE_DIRECTORY noop|fake|full")
    mode = sys.argv[2]
    exe = pathlib.Path(sys.argv[1]).resolve() / "winws2.exe"
    for name in ("winws2.exe", "zapret-lib.lua", "zapret-antidpi.lua"):
        if not (exe.parent / name).is_file():
            raise RuntimeError(f"Missing engine/Lua asset: {name}")
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    port = server.server_port
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    print(f"Lua HTTP candidate is restricted to 127.0.0.1:{port}", flush=True)
    raw = (
        f"outbound and loopback and ip and tcp and "
        f"ip.DstAddr == 127.0.0.1 and tcp.DstPort == {port}"
    )
    # The HTTP-specific part of windows_tcp_candidate_profile(). Only the
    # port changes, to the dynamically allocated isolated test port.
    # --wf-raw overrides the built-in WinDivert filter constructor.
    args = [
        str(exe),
        "--debug=1",
        "--wf-dup-check=0",
        f"--wf-raw={raw}",
        "--lua-init=@zapret-lib.lua",
        "--lua-init=@zapret-antidpi.lua",
        f"--filter-tcp={port}",
        "--filter-l7=http",
        "--payload=http_req",
    ]
    if mode == "noop":
        # Upstream luaexec() demonstrates live Lua packet callbacks while
        # leaving packets unchanged (no fake packets or altered segments).
        args.append("--lua-desync=luaexec:code=DLOG('OCHISTKA_DESYNC_CALLBACK')")
    else:
        args.append("--lua-desync=fake:blob=fake_default_http:tcp_md5")
    if mode == "full":
        args.extend([
            "--payload=http_req",
            "--lua-desync=multisplit:pos=1:seqovl=5:seqovl_pattern=0x1603030000",
        ])

    with tempfile.TemporaryDirectory(prefix="ochistka-candidate-") as tmp:
        log = pathlib.Path(tmp) / "winws2-candidate.log"
        proc: subprocess.Popen[bytes] | None = None
        try:
            with log.open("wb", buffering=0) as output:
                proc = subprocess.Popen(
                    args,
                    cwd=str(exe.parent),
                    stdout=output,
                    stderr=subprocess.STDOUT,
                    creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
                )
                wait_ready(proc, log)
                for _ in range(8):
                    request_local_http(port)
                    time.sleep(0.05)
                deadline = time.monotonic() + 7.0
                while time.monotonic() < deadline:
                    log_text = inspect_log(log, limit=250000)
                    captured = "packet: id=" in log_text
                    # These messages are emitted from the ORIGINAL upstream
                    # Lua functions only when called for the HTTP payload.
                    fake_executed = "LUA: fake:" in log_text
                    multisplit_executed = "LUA: multisplit:" in log_text
                    noop_executed = "LUA: OCHISTKA_DESYNC_CALLBACK" in log_text
                    reinjected = "reinject unmodified" in log_text
                    confirmed = (
                        (mode == "noop" and noop_executed and reinjected)
                        or (mode == "fake" and fake_executed)
                        or (mode == "full" and fake_executed and multisplit_executed)
                    )
                    if captured and confirmed:
                        label = {
                            "noop": "luaexec() no-op callback",
                            "fake": "fake()",
                            "full": "fake() + multisplit()",
                        }[mode]
                        print(
                            f"PASS: localhost HTTP succeeded; WinDivert captured "
                            f"packets; upstream {label} was executed.",
                            flush=True,
                        )
                        return 0
                    if proc.poll() is not None:
                        break
                    time.sleep(0.2)
                raise RuntimeError(
                    "Candidate HTTP request passed but required Lua action was not confirmed.\n"
                    + inspect_log(log, limit=14000)
                )
        except Exception:
            # The engine log is essential to diagnose an isolated HTTP timeout.
            # No real user traffic or secrets: the test only visits localhost.
            print("ENGINE LOG (last 16000 chars):", flush=True)
            print(inspect_log(log, limit=16000), flush=True)
            raise
        finally:
            if proc is not None and proc.poll() is None:
                try:
                    proc.send_signal(signal.CTRL_BREAK_EVENT)
                    proc.wait(timeout=4)
                except (OSError, subprocess.TimeoutExpired):
                    proc.kill()
                    proc.wait(timeout=4)
            server.shutdown()
            server.server_close()
            print("Candidate test processes stopped.", flush=True)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"FAIL: {type(exc).__name__}: {exc}", file=sys.stderr, flush=True)
        raise SystemExit(1)
