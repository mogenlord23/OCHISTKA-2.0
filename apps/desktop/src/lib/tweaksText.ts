/** Тексты настроек Windows и Boost. Идентификаторы совпадают с ядром (crates/ochistka-core/src/tweaks.rs). */

export interface TweakText {
  id: string;
  title: string;
  description: string;
}

export const SAVED_NOTE = 'Исходное состояние сохраняется перед применением';

export const SETTINGS_TWEAKS: readonly TweakText[] = [
  {
    id: 'transparency',
    title: 'Убрать прозрачность Windows',
    description: 'Отключает прозрачные поверхности Windows. Содержимое окон и обои не меняются.',
  },
  {
    id: 'web_search',
    title: 'Убрать веб-подсказки из поиска',
    description:
      'Запрашивает отключение веб-предложений в поиске Windows. Поведение зависит от выпуска и версии Windows.',
  },
  {
    id: 'spotlight',
    title: 'Убрать «Интересное» и подборки',
    description:
      'Отключает рекомендованное содержимое и подборки Windows Spotlight, в том числе на экране блокировки. Для обновления может понадобиться выход из учётной записи.',
  },
  {
    id: 'suggestions',
    title: 'Меньше предложений программ',
    description: 'Отключает рекомендуемые приложения и советы в меню «Пуск».',
  },
];

export interface ProfileText {
  id: 'basic' | 'gaming' | 'max';
  title: string;
  description: string;
}

export const BOOST_PROFILES: readonly ProfileText[] = [
  {
    id: 'basic',
    title: 'Базовый',
    description: 'Игровой режим и отключение фоновой записи. Без изменения схемы питания.',
  },
  {
    id: 'gaming',
    title: 'Игровой',
    description:
      'Базовые параметры и схема питания «Высокая производительность». Больше расход энергии.',
  },
  {
    id: 'max',
    title: 'Максимум',
    description:
      'Базовые параметры и схема «Максимальная производительность». Выше энергопотребление, следи за температурой. На некоторых компьютерах схема недоступна.',
  },
];

export const BOOST_EXTRAS: readonly TweakText[] = [
  {
    id: 'game_dvr',
    title: 'Убрать фоновую запись игр',
    description:
      'Отключает фоновую запись игр и захват клипов. Это может уменьшить нагрузку; запись через Xbox Game Bar станет недоступна до отката.',
  },
];

export const CONFIRM_TEXT =
  'Прежнее значение будет сохранено. Вернуть его можно кнопкой «Вернуть изменения раздела» или снятием галочки.';
