---
name: ochistka-ui
description: Match OCHISTKA user interface to existing design and behavior requirements
disable-model-invocation: true
---

# UI workflow

1. Read only `CLAUDE.md`, relevant parts of `docs/REQUIREMENTS.md`, `docs/PROMPT-round3.md`, `docs/design/design-system.md`, and the target Svelte component.
2. Keep the seven sections and right-side navigation; VPN opens first. Follow the latest owner decisions, not superseded early screenshots.
3. Reuse existing tokens, components (`Check.svelte`, shared outlined selection), layout measurements and icon loading patterns; do not introduce new visual language or dependencies without need.
4. Make one screen/interaction change at a time and preserve immediate selection/re-click restore. No unwanted confirm dialogs.
5. Check keyboard accessibility, contrast, reduced motion and responsive behavior. Use targeted Vitest and `svelte-check` where feasible.
6. Do not claim pixel-perfect Windows rendering without real Windows screenshots; record limitations.
