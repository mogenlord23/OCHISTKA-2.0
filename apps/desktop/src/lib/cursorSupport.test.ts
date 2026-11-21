import { describe, expect, it } from 'vitest';
import { CSS_CURSORS } from './cursorSupport';

/** Все 17 ролей схемы (совпадают с `Role::ALL` в ядре). */
const SCHEME_ROLES = [
  'Arrow',
  'Help',
  'AppStarting',
  'Wait',
  'Crosshair',
  'IBeam',
  'NWPen',
  'No',
  'SizeNS',
  'SizeWE',
  'SizeNWSE',
  'SizeNESW',
  'SizeAll',
  'UpArrow',
  'Hand',
  'Pin',
  'Person',
];

describe('css cursor support table', () => {
  it('lists every state of the cursor tester exactly once', () => {
    const wanted = [
      'default',
      'pointer',
      'text',
      'vertical-text',
      'wait',
      'progress',
      'move',
      'crosshair',
      'help',
      'not-allowed',
      'n-resize',
      's-resize',
      'e-resize',
      'w-resize',
      'ne-resize',
      'nw-resize',
      'se-resize',
      'sw-resize',
      'ns-resize',
      'ew-resize',
      'nesw-resize',
      'nwse-resize',
      'col-resize',
      'row-resize',
      'cell',
      'zoom-in',
      'zoom-out',
      'grab',
      'grabbing',
      'copy',
      'alias',
    ];
    expect(CSS_CURSORS.map((c) => c.css).sort()).toEqual([...wanted].sort());
  });

  it('maps system states to roles the scheme really draws', () => {
    for (const c of CSS_CURSORS.filter((item) => item.support !== 'browser')) {
      expect(SCHEME_ROLES, c.css).toContain(c.role);
    }
  });

  it('does not promise to replace what the browser draws itself', () => {
    const own = CSS_CURSORS.filter((c) => c.support === 'browser').map((c) => c.css);
    expect(own).toEqual(
      expect.arrayContaining(['grab', 'grabbing', 'copy', 'alias', 'zoom-in', 'zoom-out', 'cell']),
    );
  });
});
