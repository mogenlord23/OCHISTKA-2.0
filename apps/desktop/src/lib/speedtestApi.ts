/** Команды теста скорости. Запросы к серверу измерения делает Rust; здесь только вызовы. */
import { Channel, invoke } from '@tauri-apps/api/core';
import type { SpeedEvent } from './generated/SpeedEvent';

export type { SpeedEvent };
export type { SpeedFailure } from './generated/SpeedFailure';
export type { SpeedPhase } from './generated/SpeedPhase';
export type { SpeedResult } from './generated/SpeedResult';

/** Запускает тест; ход приходит в `onEvent`, промис завершается вместе с тестом. */
export function startSpeedtest(onEvent: (event: SpeedEvent) => void): Promise<SpeedEvent> {
  const channel = new Channel<SpeedEvent>();
  channel.onmessage = onEvent;
  return invoke<SpeedEvent>('speedtest_start', { onEvent: channel });
}

export const cancelSpeedtest = () => invoke<void>('speedtest_cancel');
