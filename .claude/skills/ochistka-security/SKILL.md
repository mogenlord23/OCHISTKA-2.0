---
name: ochistka-security
description: Focused safety analysis for scanner, quarantine, cleanup, privileged Windows operations
disable-model-invocation: true
---

# Security and reversible operations

1. Read `CLAUDE.md`, `docs/adr/0004-own-engine.md`, `docs/adr/0005-quarantine.md`, relevant code and tests only.
2. Never confuse an antivirus UI or heuristic score with proven malware detection; state missing signatures, benchmarks, false positives, and OS verification.
3. Keep antivirus `unsafe` constrained to the dedicated FFI boundary and expose a safe trait. Check filename normalization, allowlists, symlink/reparse points, TOCTOU, timeouts, large files and process permissions where relevant.
4. Never permanently delete a real file without separately confirmed user action. Default to quarantine, restore support, hashes and auditable manifest; no silent operations on system paths.
5. For driver changes, registry, services, firewall, proxy and power settings: preserve original state and rollback; elevation only for the individual task.
6. Add smallest regression tests that exercise risky branches. Do not start an unrestricted full-repository vulnerability audit by default.
7. Report risks and unverified assumptions explicitly.
