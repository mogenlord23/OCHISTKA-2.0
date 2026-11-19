---
name: ochistka-vpn
description: Narrow implementation or review of sing-box VPN and Windows network settings
disable-model-invocation: true
---

# VPN workflow

1. Read `CLAUDE.md`, current `docs/status/vpn.md` and `docs/status/vpn-0.5.5-behaviour.md`, plus only necessary VPN Rust/TS files.
2. Preserve the existing sing-box architecture, pinned download/hash verification, HTTPS subscription policy and input validation.
3. Connection, disconnection, crash recovery and system proxy changes require rollback and explicit user awareness. No live network changes during tests by default.
4. Differentiate system proxy from full-device TUN routing; do not claim anonymity or leak protection without targeted tests (DNS, IPv6, routing, kill switch).
5. Use test fixtures, focused unit tests and mocks first. Do not fetch unknown executables or run third-party installation scripts automatically.
6. Label clearly which scenarios have been verified on Windows and which remain untested.
