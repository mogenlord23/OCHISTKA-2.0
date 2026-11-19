import { toCommandError } from './api';
import { tweaksOverview } from './tweaksApi';
import type { CommandError, TweakState, TweaksOverview } from './types';

/**
 * Применение настроек без подтверждений и без блокировки интерфейса.
 * Переключатель меняется в тот же кадр, система применяется в фоне по очереди,
 * при ошибке он возвращается и рядом показывается короткая понятная фраза.
 */
export class TweakRunner {
  data = $state<TweaksOverview | null>(null);
  /** Ошибка чтения состояния (например, функция не поддерживается системой). */
  error = $state<CommandError | null>(null);
  /** Мгновенно показанное состояние (ключ → включено), пока работа ещё идёт. */
  optimistic = $state<Record<string, boolean>>({});
  /** Короткая фраза об ошибке по ключу строки; живёт до следующего нажатия на эту строку. */
  failed = $state<Record<string, string>>({});
  /** Сколько заданий по ключу строки ещё не закончено (ожидание на переключателе). */
  inflight = $state<Record<string, number>>({});
  /** Сколько заданий в очереди или выполняется. */
  pending = $state(0);

  private tokens: Record<string, number> = {};
  private seq = 0;
  private chain: Promise<void> = Promise.resolve();

  /** Что показывать в переключателе: мгновенное значение, если оно есть, иначе настоящее. */
  shown(key: string, actual: boolean): boolean {
    return key in this.optimistic ? this.optimistic[key] : actual;
  }

  isPending(key: string): boolean {
    return (this.inflight[key] ?? 0) > 0;
  }

  /** Состояние настройки из ядра (доступность, причина, перезапуск). */
  state(id: string): TweakState | undefined {
    return this.data?.tweaks.find((tweak) => tweak.id === id);
  }

  refresh(): Promise<void> {
    return this.enqueue({}, tweaksOverview);
  }

  /** Задание с мгновенным показом: `shownNow` сразу попадает в переключатели, `job` идёт в очередь. */
  apply(shownNow: Record<string, boolean>, job: () => Promise<TweaksOverview>): Promise<void> {
    return this.enqueue(shownNow, job);
  }

  private enqueue(
    shownNow: Record<string, boolean>,
    job: () => Promise<TweaksOverview>,
  ): Promise<void> {
    const token = ++this.seq;
    const keys = Object.keys(shownNow);
    for (const [key, value] of Object.entries(shownNow)) {
      this.tokens[key] = token;
      this.optimistic[key] = value;
      delete this.failed[key];
      this.inflight[key] = (this.inflight[key] ?? 0) + 1;
    }
    this.pending += 1;
    const run = async () => {
      try {
        this.data = await job();
        this.error = null;
      } catch (err) {
        const error = toCommandError(err);
        if (keys.length === 0 || error.kind === 'unsupported') {
          this.error = error;
        } else {
          const phrase = tweakErrorText(error, '');
          for (const key of keys) this.failed[key] = phrase;
        }
      } finally {
        this.pending -= 1;
        for (const key of keys) {
          this.inflight[key] = Math.max(0, (this.inflight[key] ?? 1) - 1);
          // Снимаем мгновенное значение, только если новее нажатия не было.
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
  if (error.kind === 'needsAdmin') return 'Нужны права администратора.';
  if (error.kind === 'notApplicable' && error.message) return error.message;
  return 'Не получилось изменить настройку. Попробуй ещё раз.';
}
