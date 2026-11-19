/** Тексты настроек Windows и Boost. Идентификаторы совпадают с ядром (crates/ochistka-core/src/tweaks_catalog.rs);
    таблица ключей реестра: docs/status/settings-keys.md. */

export interface TweakText {
  id: string;
  title: string;
  description: string;
}

export interface TweakCategory {
  id: string;
  title: string;
  tweaks: readonly TweakText[];
}

/** Служебная строка: показывается один раз вверху раздела, не в каждой строке. */
export const SAVED_NOTE =
  'Исходное состояние сохраняется перед применением. Любую настройку можно вернуть.';

export const SETTINGS_CATEGORIES: readonly TweakCategory[] = [
  {
    id: 'look',
    title: 'Внешний вид и рекомендации Windows',
    tweaks: [
      {
        id: 'transparency',
        title: 'Убрать прозрачность Windows',
        description: 'Отключает прозрачные поверхности. Окна и обои не меняются.',
      },
      {
        id: 'spotlight',
        title: 'Советы и «Интересное» на экране блокировки',
        description: 'Отключает подборки Windows Spotlight и советы на экране блокировки.',
      },
      {
        id: 'login_blur',
        title: 'Размытие при входе',
        description: 'Фон экрана входа без размытия.',
      },
      {
        id: 'widgets',
        title: 'Виджеты на панели задач',
        description: 'Убирает виджеты и «Новости и интересы» с панели задач.',
      },
      {
        id: 'fewer_offers',
        title: 'Меньше персональных предложений',
        description: 'Windows не подбирает советы и рекламу по данным диагностики.',
      },
      {
        id: 'extra_tips',
        title: 'Дополнительные подсказки Windows',
        description: 'Без «давайте завершим настройку» и советов после обновлений.',
      },
      {
        id: 'ads',
        title: 'Рекламные предложения',
        description: 'Не ставит рекомендованные приложения и не показывает рекламу в системе.',
      },
      {
        id: 'suggestions',
        title: 'Рекомендации в «Пуске»',
        description: 'Убирает предложенные программы и недавние рекомендации в меню «Пуск».',
      },
      {
        id: 'new_outlook',
        title: 'Не предлагать новый Outlook',
        description: 'Оставляет привычный Outlook и отключает автоматический переход.',
      },
    ],
  },
  {
    id: 'search',
    title: 'Поиск Windows',
    tweaks: [
      {
        id: 'web_search',
        title: 'Убрать веб-подсказки из поиска',
        description: 'Поиск на панели задач ищет только на компьютере, без запросов в интернет.',
      },
      {
        id: 'search_highlights',
        title: '«Интересное» в поиске',
        description: 'Убирает картинки и события дня в окне поиска.',
      },
      {
        id: 'cloud_search',
        title: 'Облачный поиск',
        description: 'Поиск не заглядывает в учётную запись Microsoft и рабочую учётную запись.',
      },
    ],
  },
  {
    id: 'privacy',
    title: 'Конфиденциальность и диагностика',
    tweaks: [
      {
        id: 'diagnostics',
        title: 'Минимум диагностики',
        description: 'Windows отправляет только обязательные сведения.',
      },
      {
        id: 'ad_id',
        title: 'Рекламный ID',
        description: 'Приложения не получают рекламный идентификатор.',
      },
      {
        id: 'location',
        title: 'Геолокация для приложений',
        description: 'Запрещает приложениям определять местоположение.',
      },
      {
        id: 'error_reports',
        title: 'Отчёты об ошибках',
        description: 'Windows не отправляет отчёты о сбоях в Microsoft.',
      },
      {
        id: 'feedback',
        title: 'Запросы отзывов',
        description: 'Windows не просит оценить систему.',
      },
      {
        id: 'recall',
        title: 'Снимки Recall',
        description: 'Запрещает анализ экрана и сохранение снимков (Recall).',
      },
      {
        id: 'camera',
        title: 'Камера для приложений',
        description: 'Запрещает приложениям пользоваться камерой.',
      },
      {
        id: 'microphone',
        title: 'Микрофон для приложений',
        description: 'Запрещает приложениям пользоваться микрофоном.',
      },
      {
        id: 'contacts',
        title: 'Контакты для приложений',
        description: 'Запрещает приложениям читать контакты.',
      },
    ],
  },
  {
    id: 'edge',
    title: 'Microsoft Edge',
    tweaks: [
      {
        id: 'edge_background',
        title: 'Не оставлять Edge в фоне',
        description: 'После закрытия окна Edge полностью выгружается из памяти.',
      },
      {
        id: 'edge_telemetry',
        title: 'Не отправлять сведения о сайтах',
        description: 'Edge не передаёт данные об использовании и посещённых сайтах.',
      },
      {
        id: 'edge_ads',
        title: 'Персональная реклама в Edge',
        description: 'Отключает подбор рекламы по истории просмотров.',
      },
      {
        id: 'edge_suggest',
        title: 'Предложения поиска в Edge',
        description: 'Строка адреса не отправляет введённый текст для подсказок.',
      },
      {
        id: 'edge_passwords',
        title: 'Не сохранять новые пароли',
        description: 'Edge перестаёт предлагать запомнить пароль.',
      },
      {
        id: 'edge_sidebar',
        title: 'Боковая панель и Copilot в Edge',
        description: 'Убирает боковую панель с Copilot и сервисами.',
      },
    ],
  },
  {
    id: 'cloud',
    title: 'Системные компоненты и облако',
    tweaks: [
      {
        id: 'cortana',
        title: 'Cortana',
        description: 'Отключает Cortana и её кнопку (есть только в Windows 10).',
      },
      {
        id: 'copilot',
        title: 'Windows Copilot',
        description: 'Отключает Copilot и убирает его кнопку с панели задач.',
      },
      {
        id: 'onedrive_sync',
        title: 'Синхронизация OneDrive',
        description: 'Запрещает OneDrive синхронизировать файлы.',
      },
      {
        id: 'settings_sync',
        title: 'Синхронизация параметров профиля',
        description: 'Пароли, темы и настройки не синхронизируются между компьютерами.',
      },
    ],
  },
  {
    id: 'updates',
    title: 'Обновления Windows',
    tweaks: [
      {
        id: 'update_ask',
        title: 'Спрашивать перед загрузкой обновлений',
        description: 'Обновления скачиваются только после твоего согласия.',
      },
      {
        id: 'no_auto_reboot',
        title: 'Не перезагружать при активном сеансе',
        description: 'Windows не перезагружается сама, пока ты за компьютером.',
      },
    ],
  },
];

export const SETTINGS_TWEAKS: readonly TweakText[] = SETTINGS_CATEGORIES.flatMap(
  (category) => category.tweaks,
);

export interface ProfileText {
  id: 'basic' | 'gaming' | 'max';
  title: string;
  description: string;
}

export const BOOST_PROFILES: readonly ProfileText[] = [
  {
    id: 'basic',
    title: 'Базовый',
    description:
      'Отключает фоновую запись игр и включает игровой режим Windows. Схема питания не меняется.',
  },
  {
    id: 'gaming',
    title: 'Игровой',
    description:
      'Базовые параметры и схема питания «Высокая производительность». Расход энергии выше.',
  },
  {
    id: 'max',
    title: 'Максимум',
    description:
      'Базовые параметры, схема «Максимальная производительность» и отказ от ограничения фоновой мощности (это требует прав администратора). Если схемы нет на компьютере или нет прав, пункт пропускается, остальное применяется.',
  },
];

export const BOOST_EXTRAS: readonly TweakText[] = [
  {
    id: 'game_dvr',
    title: 'Убрать фоновую запись игр',
    description:
      'Выключает Game DVR и захват клипов. Запись через Xbox Game Bar станет недоступна.',
  },
  {
    id: 'game_dvr_policy',
    title: 'Запретить запись игр для всей системы',
    description: 'Политика Windows: запись не включится, даже если её включить в Xbox Game Bar.',
  },
  {
    id: 'game_mode',
    title: 'Игровой режим Windows',
    description:
      'Windows отдаёт игре больше ресурсов и откладывает установку драйверов во время игры.',
  },
  {
    id: 'power_throttling',
    title: 'Без ограничения фоновой мощности',
    description: 'Отключает Power Throttling: процессы не замедляются в фоне. Расход энергии выше.',
  },
  {
    id: 'ultimate',
    title: 'Ultimate Performance',
    description:
      'Схема электропитания «Максимальная производительность». Прежняя схема запоминается.',
  },
  {
    id: 'hags',
    title: 'Планирование GPU (HAGS)',
    description: 'Видеокарта сама планирует задачи. Нужна поддержка видеокарты и драйвера.',
  },
  {
    id: 'vbs_off',
    title: 'Игровой режим без VBS/HVCI',
    description:
      'Отключает изоляцию ядра (VBS/HVCI): игры быстрее, но защита Windows от вредоносного кода слабее.',
  },
];

/** Названия пунктов профиля для списка «что включено» (`power` — схема питания). */
export function profileStepTitle(id: string): string {
  if (id === 'power') return 'Схема электропитания';
  return BOOST_EXTRAS.find((tweak) => tweak.id === id)?.title ?? id;
}

/** Когда изменение вступает в силу (`restart` из ядра); пустая строка — сразу. */
export function restartText(restart: string): string {
  switch (restart) {
    case 'explorer':
      return 'после перезапуска Проводника или выхода из системы';
    case 'signOut':
      return 'после выхода из системы';
    case 'reboot':
      return 'после перезагрузки';
    default:
      return '';
  }
}

export const VBS_CONFIRM = {
  title: 'Отключить защиту ядра Windows?',
  body: 'Изоляция ядра (VBS/HVCI) защищает систему от вредоносных драйверов. Без неё игры могут работать быстрее, но защита станет слабее. Изменение вступит в силу после перезагрузки. Прежнее значение сохранится: настройку можно выключить, и защита вернётся.',
  ok: 'Отключить',
  cancel: 'Отмена',
} as const;

export const ACTIVATION_TEXT: Record<string, string> = {
  licensed: 'Windows активирована',
  notActivated: 'Windows не активирована',
  grace: 'Windows активирована временно',
  notification: 'Windows требует активации',
  unknown: 'Состояние активации не удалось определить',
};
