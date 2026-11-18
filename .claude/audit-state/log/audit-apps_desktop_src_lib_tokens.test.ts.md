File: apps/desktop/src/lib/tokens.test.ts
Declared Lines: 93
Lines Inspected: 93 (tokens.ts 69 lines also read; hand verification of channel, luminance, contrastRatio, textOn)
Functions Analyzed: 10 tests; 4 functions under test hand-traced

Execution Traces: formula constants match sRGB within rounding (0.03928 versus 0.04045 indistinguishable for integer channels). Hand-computed ratios for all seven section colours and neutral pairs match docs/design/accessibility-2026-11-18.md. Black and white endpoints (21:1) and 4.5 AA minimum for all section colours are asserted. Findings: circular AA oracle, mid-tone curve unpinned (FND-0204, low); misleading black-text test (FND-0205); no neutral or UI contrast test (FND-0206, low); brief doc-drift (FND-0207); restating tests (FND-0208); font weak assertions (FND-0209).
Cross-references (not re-filed): FND-0019, FND-0022-adjacent, FND-0023, FND-0082, FND-0083, FND-0084, FND-0085, FND-0086, FND-0087, FND-0122, FND-0203.
Not verified: vitest not run (all figures hand calculations); background 7% pattern can lower real contrast below the flat-colour figures (FND-0122); UI usage of NEUTRAL pairs for 3:1 compliance out of scope.

STATUS: COMPLETE
(Recorded by the orchestrator from the auditor's read-only report.)
