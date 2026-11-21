# Focused visual acceptance checklist (read only the relevant block)

## Common
- Test true currently supported minimum viewport + representative medium and maximum. Check overflow, stacking contexts, pointer targets, clipping, and CSS box measurements.
- Shared source: `tokens.ts`, reusable components, eight-pixel grid when required; do not invent global radii or shadows.
- Validate keyboard focus and `prefers-reduced-motion`; compare stable screenshots without dynamic clock/graphs noise.

## VPN
- Poster, server list and connection panels align to common edges at supported sizes; main window must not unexpectedly scroll on poster wheel.
- Sound/language dock covers rail highlights in small/large windows; poster rounded edges have no dark bleed-through.
- Tab change preserves speed-test state; buttons reflect real connection state rather than falsely reporting a connected VPN.

## Appearance
- Cursor selection responds on current frame, system apply is independently acknowledged; no rotating preview cards.
- Wallpaper thumbnail outlines have no dark halo; re-click / restore semantics preserved.
- Colors row of three black controls (transparency, taskbar+Start fill, restore Windows); palette never flickers due to component remount.

## Computer / Boost
- Information shell renders immediately with cache/neutral placeholders and never fabricates telemetry.
- Toggling an option does not reset list scroll or move window; real Windows effect is tested separately from UI selection.

## Processes / Programs / Disks
- Long lists fill available content height without stretching rows; their own scroll works.
- Context menu anchored close to pointer and clamped to viewport; no WebView browser menu outside editable controls.

## Evidence / limitations
- Report measured viewport, target, screenshots, DOM geometry and test result, exact approved reference if available.
- Visual mock PASS is not native Windows verified. If no renderer is available, label VISUAL NOT VERIFIED.
