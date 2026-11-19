import { IDLE, START, reduce, markNoteSeen, type SpeedUi } from './speedtest';
import { cancelSpeedtest, startSpeedtest } from './speedtestApi';

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
    const apply = (event: Parameters<typeof reduce>[1]) => {
      this.ui = reduce(this.ui, event);
      if (this.ui.status === 'done') this.last = this.ui;
    };
    try {
      await startSpeedtest(apply);
      // Если Rust завершился, не прислав итога, не оставляем кнопку в ожидании.
      if (this.ui.status === 'running') apply({ type: 'failed', kind: 'server' });
    } catch {
      apply({ type: 'failed', kind: 'server' });
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
