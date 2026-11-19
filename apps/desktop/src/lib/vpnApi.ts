/** Команды раздела VPN. Сеть, процессы и реестр делает Rust; здесь только вызовы. */
import { invoke } from '@tauri-apps/api/core';
import type { ConnectionStatus } from './generated/VpnConnection';
import type { ImportSummary } from './generated/VpnImportSummary';
import type { RoutingRules } from './generated/VpnRules';
import type { VpnSnapshot } from './generated/VpnSnapshot';

export type { ConnectionStatus, ImportSummary, RoutingRules, VpnSnapshot };
export type { CoreAvailability } from './generated/VpnCoreStatus';
export type { PresetInfo } from './generated/VpnPreset';
export type { ServerView } from './generated/VpnServer';

export const vpnSnapshot = () => invoke<VpnSnapshot>('vpn_snapshot');
export const vpnStatus = () => invoke<ConnectionStatus>('vpn_status');
/** Ссылка https:// подписки, вставленный список ссылок или текст файла. */
export const vpnImport = (input: string) => invoke<ImportSummary>('vpn_import', { input });
export const vpnSelect = (id: string) => invoke<void>('vpn_select', { id });
export const vpnSetRules = (rules: RoutingRules) =>
  invoke<RoutingRules>('vpn_set_rules', { rules });
export const vpnMeasureLatency = () => invoke<VpnSnapshot>('vpn_measure_latency');
export const vpnConnect = () => invoke<ConnectionStatus>('vpn_connect');
export const vpnDisconnect = () => invoke<ConnectionStatus>('vpn_disconnect');
