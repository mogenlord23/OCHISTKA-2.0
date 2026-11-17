File: crates/platform/src/lib.rs
Declared Lines: 97
Lines Inspected: 97
Functions/Methods Analyzed: 8 (3 cfg variants of current_system_info, 2 of current_virus_scanner, test helper, 4 tests)

Execution Traces:
- current_system_info (windows): WindowsSystemInfo::new() no I/O [none]
- current_system_info (macos): MacosSystemInfo Unsupported [none]
- current_system_info (other): LinuxSystemInfo Unsupported; FreeBSD would say Linux, cosmetic, not filed [none]
- current_virus_scanner (windows): current_exe().ok() -> from_install_dir is_file check; io::Error discarded [FND-0015]
- current_virus_scanner (other): unavailable() [FND-0003 applies]
- tests: [FND-0013, FND-0014]

Documentation drift: FND-0016 (installer does not bundle ClamAV; reason text differs).
Missing feature, not a defect: freshclam is required by CLAUDE.md and ADR 0002 but has no code in the repo; not filed.
UNVERIFIED: windows/mod.rs:39 System::new() Result API assumed from sysinfo 0.40.4; confirmed in an earlier build but not re-read in this audit.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
