File: crates/platform/src/windows/mod.rs
Declared Lines: 80
Lines Inspected: 80 (sysinfo sources traced read-only)
Functions Analyzed: 5 (new, default, probe_error, init_system, overview)

Execution Traces: new no I/O; default delegates; probe_error unreachable on Windows; init_system sampling and sleep (lock held, correct retry on error); overview: lock poisoning (FND-0037), zero-on-failure (FND-0038), disk enumeration under lock (FND-0039), unreachable branch (FND-0042). Callers: commands.rs:52-62 via spawn_blocking; Home polls every 2 s.
Observations: FND-0040 (IOCTL, UNVERIFIED), FND-0041 (coverage), FND-0043 (doc drift).
Not verified: PDH open time; removable device blocking time; IOCTL semantics; sysinfo panic paths beyond disk.rs:30-33.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
