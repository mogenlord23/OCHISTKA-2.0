File: apps/desktop/tsconfig.json
Declared Lines: 15
Lines Inspected: 15
Functions Analyzed: 0 (JSON object, 2 top-level keys)

Execution Traces: no extends, so no inherited options (single tsconfig in repo). target, module, moduleResolution, strict, skipLibCheck, noEmit are valid; no tsc script exists. types and lib mix Node and DOM globals across one program (FND-0145). Version split: svelte-check loads TS 6.0.3 (auto-installed peer) while the declared compiler is TS 7.0.2 (FND-0146). isolatedModules is implied by verbatimModuleSyntax (FND-0147). Cross-references: FND-0088, FND-0089 (vite/client loaded twice, confirmed), FND-0143 (svelte.config.js not included, confirmed).
UNVERIFIED: svelte-check and tsc were not run; Vite 8 define handling of process.env in client builds; TS 7 option handling (checked only against TS 6.0.3 source); TS 6 default changes against src imports.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
