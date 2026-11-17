File: apps/desktop/src/lib/types.ts
Declared Lines: 57
Lines Inspected: 57
Items Analyzed: 10 declarations

Execution Traces: every interface matches its Rust DTO by name, optionality and type (SystemOverview, CpuStatus, MemoryStatus, DiskStatus, CommandError, Finding, ScanReport, EngineStatus, Suspicion). ErrorKind matches the four CoreError kinds that commands.rs can emit. No Rust DTO missing; FileFacts correctly absent (no Serialize).
Findings: FND-0090 (closed union vs free String, info), FND-0091 (u64 precision, info).
Already covered: FND-0029, FND-0058, FND-0060, FND-0067, FND-0074.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
