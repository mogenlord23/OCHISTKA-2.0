import { IDLE, START, reduce, markNoteSeen, type SpeedUi } from './speedtest';
import { cancelSpeedtest, startSpeedtest, type SpeedEvent } from './speedtestApi';

/**
 * Состояние проверки скорости живёт здесь, а не в странице: переход по разделам тест не
 * прерывает, ход и последний завершённый результат при возврате на месте. Новый запуск
 * только по нажатию.
 */
export class SpeedStore {
  ui = $state<SpeedUi>(IDLE);
  /** Последний завершённый результат: остаётся видимым после ошибки или остановки нового теста. */
  last = $state<SpeedUi | null>(null);

  get running(): boolean {
    return this.ui.status === 'running';
  }

  /** Что показывать в карточках: ход теста, итог или (после сбоя) прошлый итог. */
  get view(): SpeedUi {
    const keepLast = (this.ui.status === 'failed' || this.ui.status === 'cancelled') && this.last;
    return keepLast ? this.last! : this.ui;
  }

  async start(): Promise<void> {
    if (this.running) return;
    markNoteSeen();
    this.ui = START;
    let acceptingEvents = true;
    const apply = (event: SpeedEvent) => {
      if (acceptingEvents) this.ui = reduce(this.ui, event);
    };
    try {
      const terminal = await startSpeedtest(apply);
      acceptingEvents = false;
      if (!terminal || !['done', 'failed', 'cancelled'].includes(terminal.type)) {
        throw new Error('Тест завершился без результата');
      }
      this.ui = reduce(this.ui, terminal);
      if (this.ui.status === 'done') this.last = this.ui;
    } catch {
      acceptingEvents = false;
      this.ui = reduce(this.ui, { type: 'failed', kind: 'server' });
    }
  }

  stop(): void {
    void cancelSpeedtest().catch(() => {});
  }

  /** Для тестов. */
  reset(): void {
    this.ui = IDLE;
    this.last = null;
  }
}

export const speedStore = new SpeedStore();
