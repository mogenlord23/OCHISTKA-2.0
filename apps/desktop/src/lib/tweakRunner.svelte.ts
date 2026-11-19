import { toCommandError } from './api';
import { tweaksOverview } from './tweaksApi';
import type { CommandError, TweaksOverview } from './types';

/**
 * Применение настроек без подтверждений и без блокировки интерфейса.
 * Галочка (обводка) меняется в тот же кадр, система применяется в фоне по очереди,
 * при ошибке галочка возвращается и показывается короткая понятная фраза.
 */
export class TweakRunner {
  data = $state<TweaksOverview | null>(null);
  error = $state<CommandError | null>(null);
  /** Мгновенно показанное состояние (ключ → включено), пока работа ещё идёт. */
  optimistic = $state<Record<string, boolean>>({});
  /** Сколько заданий в очереди или выполняется. */
  pending = $state(0);

  private tokens: Record<string, number> = {};
  private seq = 0;
  private chain: Promise<void> = Promise.resolve();

  /** Что показывать в галочке: мгновенное значение, если оно есть, иначе настоящее. */
  shown(key: string, actual: boolean): boolean {
    return key in this.optimistic ? this.optimistic[key] : actual;
  }

  refresh(): Promise<void> {
    return this.enqueue({}, tweaksOverview);
  }

  /** Задание с мгновенным показом: `shownNow` сразу попадает в галочки, `job` идёт в очередь. */
  apply(shownNow: Record<string, boolean>, job: () => Promise<TweaksOverview>): Promise<void> {
    return this.enqueue(shownNow, job);
  }

  private enqueue(
    shownNow: Record<string, boolean>,
    job: () => Promise<TweaksOverview>,
  ): Promise<void> {
    const token = ++this.seq;
    for (const [key, value] of Object.entries(shownNow)) {
      this.tokens[key] = token;
      this.optimistic[key] = value;
    }
    this.pending += 1;
    const run = async () => {
      try {
        this.data = await job();
        this.error = null;
      } catch (err) {
        this.error = toCommandError(err);
      } finally {
        this.pending -= 1;
        // Снимаем мгновенное значение, только если новее нажатия не было.
        for (const key of Object.keys(shownNow)) {
          if (this.tokens[key] === token) delete this.optimistic[key];
        }
      }
    };
    this.chain = this.chain.then(run, run);
    return this.chain;
  }
}

/** Фраза об ошибке без технических слов. */
export function tweakErrorText(error: CommandError | null, unsupportedText: string): string {
  if (!error) return '';
  if (error.kind === 'unsupported') return unsupportedText;
  return 'Не получилось изменить настройку. Попробуй ещё раз.';
}
