import { describe, expect, it } from 'vitest';
import {
  blockReason,
  filterServices,
  isPending,
  needsConfirm,
  NEEDS_ADMIN_NOTE,
  resultNote,
  startOptionValue,
  stateLabel,
} from './services';
import type { ServiceInfo } from './servicesApi';

function svc(patch: Partial<ServiceInfo> = {}): ServiceInfo {
  return {
    name: 'Fax',
    displayName: 'Факс',
    state: 'stopped',
    startType: 'manual',
    previousStartType: null,
    guard: 'free',
    note: null,
    ...patch,
  };
}

describe('services helpers', () => {
  it('labels states in Russian', () => {
    expect(stateLabel('running')).toBe('Работает');
    expect(stateLabel('stopPending')).toBe('Останавливается');
    expect(isPending('startPending')).toBe(true);
    expect(isPending('running')).toBe(false);
  });

  it('maps delayed auto start to the auto option', () => {
    expect(startOptionValue('autoDelayed')).toBe('auto');
    expect(startOptionValue('manual')).toBe('manual');
    expect(startOptionValue('boot')).toBeNull();
  });

  it('asks confirmation only for careful services on stop or disable', () => {
    const careful = svc({ guard: 'careful', note: 'Печать остановится.' });
    expect(needsConfirm(careful, 'stop')).toBe(true);
    expect(needsConfirm(careful, 'disabled')).toBe(true);
    expect(needsConfirm(careful, 'manual')).toBe(false);
    expect(needsConfirm(svc(), 'stop')).toBe(false);
  });

  it('gives a reason for locked services and for missing admin rights', () => {
    expect(blockReason(svc({ guard: 'locked', note: 'Критичная служба.' }), true)).toBe(
      'Критичная служба.',
    );
    expect(blockReason(svc(), false)).toBe(NEEDS_ADMIN_NOTE);
    expect(blockReason(svc(), true)).toBeNull();
  });

  it('filters by name, display name and running state', () => {
    const list = [svc(), svc({ name: 'Spooler', displayName: 'Печать', state: 'running' })];
    expect(filterServices(list, 'печ', false).map((s) => s.name)).toEqual(['Spooler']);
    expect(filterServices(list, 'FAX', false).map((s) => s.name)).toEqual(['Fax']);
    expect(filterServices(list, '', true).map((s) => s.name)).toEqual(['Spooler']);
  });

  it('result note reports the actual state', () => {
    expect(resultNote(svc({ state: 'running', startType: 'auto' }))).toContain('работает');
  });
});
