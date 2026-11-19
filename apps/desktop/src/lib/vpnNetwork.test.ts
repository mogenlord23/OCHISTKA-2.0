import { describe, expect, it } from 'vitest';
import {
  connectionKey,
  connectionMenuItems,
  endpointText,
  formatEndpoint,
  formatRate,
  nextSort,
  sortConnections,
  type TcpConnection,
} from './vpnNetwork';

function conn(over: Partial<TcpConnection>): TcpConnection {
  return {
    localAddr: '127.0.0.1',
    localPort: 80,
    remoteAddr: '0.0.0.0',
    remotePort: 0,
    state: 'LISTEN',
    pid: 1,
    processName: 'a.exe',
    ...over,
  };
}

describe('formatRate', () => {
  it('uses human units per second', () => {
    expect(formatRate(0)).toBe('0 Б/с');
    expect(formatRate(1536)).toBe('1.5 КБ/с');
    expect(formatRate(3 * 1024 * 1024)).toBe('3.0 МБ/с');
  });
});

describe('formatEndpoint', () => {
  it('brackets IPv6', () => {
    expect(formatEndpoint('10.0.0.1', 443)).toBe('10.0.0.1:443');
    expect(formatEndpoint('::1', 443)).toBe('[::1]:443');
  });
});

describe('nextSort', () => {
  it('toggles direction on the same column and resets on another', () => {
    const a = nextSort({ key: 'pid', direction: 'asc' }, 'pid');
    expect(a).toEqual({ key: 'pid', direction: 'desc' });
    expect(nextSort(a, 'pid').direction).toBe('asc');
    expect(nextSort(a, 'state')).toEqual({ key: 'state', direction: 'asc' });
  });
});

describe('sortConnections', () => {
  const list = [
    conn({ pid: 30, localPort: 9, processName: 'zeta.exe', state: 'ESTABLISHED' }),
    conn({ pid: 4, localPort: 100, processName: 'alpha.exe', state: 'LISTEN' }),
    conn({ pid: null, localPort: 20, processName: null, state: 'TIME_WAIT' }),
  ];

  it('sorts PIDs numerically, both ways, without mutating the input', () => {
    const asc = sortConnections(list, { key: 'pid', direction: 'asc' });
    expect(asc.map((c) => c.pid)).toEqual([null, 4, 30]);
    expect(sortConnections(list, { key: 'pid', direction: 'desc' }).map((c) => c.pid)).toEqual([
      30,
      4,
      null,
    ]);
    expect(list[0].pid).toBe(30);
  });

  it('sorts by process name and state as text', () => {
    expect(
      sortConnections(list, { key: 'process', direction: 'asc' }).map((c) => c.processName),
    ).toEqual([null, 'alpha.exe', 'zeta.exe']);
    expect(sortConnections(list, { key: 'state', direction: 'asc' })[0].state).toBe('ESTABLISHED');
  });

  it('sorts local endpoints by port when the address is equal', () => {
    expect(
      sortConnections(list, { key: 'local', direction: 'asc' }).map((c) => c.localPort),
    ).toEqual([9, 20, 100]);
  });
});

describe('соединения: ключ и действия меню', () => {
  it('key is stable and distinguishes endpoints and pid', () => {
    const a = conn({ localPort: 1, remotePort: 2 });
    expect(connectionKey(a)).toBe(connectionKey({ ...a }));
    expect(connectionKey(a)).not.toBe(connectionKey({ ...a, remotePort: 3 }));
    expect(connectionKey(a)).not.toBe(connectionKey({ ...a, pid: 9 }));
  });

  it('offers only real actions; process actions need a known process', () => {
    const withProcess = connectionMenuItems(conn({ pid: 42 }));
    expect(withProcess.map((i) => i.id)).toEqual([
      'copy-remote',
      'copy-local',
      'show-process',
      'open-location',
    ]);
    expect(withProcess.every((i) => !i.disabled)).toBe(true);
    const without = connectionMenuItems(conn({ pid: null }));
    expect(without.filter((i) => i.disabled).map((i) => i.id)).toEqual([
      'show-process',
      'open-location',
    ]);
    expect(without.find((i) => i.id === 'show-process')?.reason).toBeTruthy();
    expect(
      connectionMenuItems(conn({ pid: 0 })).find((i) => i.id === 'open-location')?.disabled,
    ).toBe(true);
  });

  it('copies the address with brackets for IPv6', () => {
    expect(endpointText(conn({ remoteAddr: '1.2.3.4', remotePort: 443 }), 'remote')).toBe(
      '1.2.3.4:443',
    );
    expect(endpointText(conn({ localAddr: '::1', localPort: 8 }), 'local')).toBe('[::1]:8');
  });
});
