File: apps/desktop/src/App.svelte
Declared Lines: 208
Lines Inspected: 208 (script 1-24, markup 26-99, style 101-208)
Functions/Methods Analyzed: 4 plus traced child components (props contract only for ComputerPage and VpnPage)

Execution Traces: imports resolve; initial state vpn/subscription; select() resets subtab; page switch unmounts previous page (FND-0018); rail/focus CSS (FND-0019); sound toggle dead (FND-0020); body font var scope (FND-0021); literals (FND-0023); VERSION copy (FND-0024); CLAUDE.md order drift (FND-0022).
UNVERIFIED: backend running-scan guard (FND-0018); TitleBar promises from @tauri-apps/api not awaited (belongs to TitleBar audit).
Test gaps noted: no tests for sound toggle, section-switch state, or the "Подразделы уточняются" branch.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
