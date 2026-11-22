/** Подпись поля ссылки подписки с печатным эффектом: «Ссылка подписки» стирается по знаку,
    печатается «Подписка сохранена», через 3 секунды обратно. Один таймер за раз; новый запуск
    отменяет прежний. Вне эффекта таймеров нет. */

export const LABEL = 'Ссылка подписки';
export const SAVED = 'Подписка сохранена';

/** Пауза на знак при стирании подписи и сохранённого текста. */
export const ERASE_MS = 35;
/** Пауза на знак при печати. */
export const TYPE_MS = 45;
/** Сколько «Подписка сохранена» остаётся на экране. */
export const HOLD_MS = 3000;

export interface LabelTypewriter {
  /** После успешной вставки ссылки: стереть подпись, напечатать «Подписка сохранена», через 3 с
      вернуть «Ссылка подписки». */
  play(): void;
  /** При открытии с уже сохранённой подпиской: сразу «Подписка сохранена», без эффекта; через 3 с
      так же сразу «Ссылка подписки». */
  showSaved(): void;
  /** Остановить всё и вернуть обычную подпись. */
  reset(): void;
}

export function createLabelTypewriter(set: (text: string) => void): LabelTypewriter {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let run = 0;

  function cancel() {
    run += 1;
    if (timer !== null) clearTimeout(timer);
    timer = null;
  }

  /** Стирает `from` по одному знаку, затем печатает `to`, затем вызывает `done`. */
  function replace(from: string, to: string, token: number, done: () => void) {
    let text = from;
    const erase = () => {
      if (token !== run) return;
      if (text.length > 0) {
        text = text.slice(0, -1);
        set(text);
        timer = setTimeout(erase, ERASE_MS);
      } else {
        type();
      }
    };
    const type = () => {
      if (token !== run) return;
      if (text.length < to.length) {
        text = to.slice(0, text.length + 1);
        set(text);
        timer = setTimeout(type, TYPE_MS);
      } else {
        done();
      }
    };
    timer = setTimeout(erase, ERASE_MS);
  }

  return {
    play() {
      cancel();
      const token = run;
      replace(LABEL, SAVED, token, () => {
        timer = setTimeout(() => {
          if (token !== run) return;
          replace(SAVED, LABEL, token, () => {});
        }, HOLD_MS);
      });
    },
    showSaved() {
      cancel();
      const token = run;
      set(SAVED);
      timer = setTimeout(() => {
        if (token === run) set(LABEL);
      }, HOLD_MS);
    },
    reset() {
      cancel();
      set(LABEL);
    },
  };
}
