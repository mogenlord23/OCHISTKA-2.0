/** Готовый каталог драйверов: только справочные данные и ссылки на официальные страницы.
 * Приложение ничего не скачивает и не устанавливает. */

import type { HardwareOverview } from './types';

export type VendorId = 'nvidia' | 'amd' | 'intel' | 'realtek' | 'bluetooth' | 'usb';

export interface Vendor {
  id: VendorId;
  label: string;
  /** Цвет плашки. */
  color: string;
  /** Короткий текст на плашке. */
  mark: string;
}

export const VENDORS: Record<VendorId, Vendor> = {
  nvidia: { id: 'nvidia', label: 'Видеокарта NVIDIA', color: '#76b900', mark: 'NVIDIA' },
  amd: { id: 'amd', label: 'AMD · видео / чипсет', color: '#ed1c24', mark: 'AMD' },
  intel: { id: 'intel', label: 'Intel · видео / чипсет', color: '#0071c5', mark: 'intel' },
  realtek: { id: 'realtek', label: 'Realtek · звук / сеть', color: '#0093d0', mark: 'R' },
  bluetooth: { id: 'bluetooth', label: 'Wi-Fi / Bluetooth', color: '#0a82ff', mark: 'ᛒ' },
  usb: { id: 'usb', label: 'USB / контроллеры', color: '#ff00d4', mark: 'USB' },
};

export interface CatalogItem {
  id: string;
  vendor: VendorId;
  title: string;
  description: string;
  /** Официальная страница производителя. */
  url: string;
}

export const CATALOG: readonly CatalogItem[] = [
  {
    id: 'nvidia-app',
    vendor: 'nvidia',
    title: 'NVIDIA App · драйверы и библиотека игр',
    description:
      'Официальное приложение NVIDIA: драйверы Game Ready и Studio, настройки игр и запись игрового процесса.',
    url: 'https://www.nvidia.com/en-us/software/nvidia-app/',
  },
  {
    id: 'nvidia-geforce',
    vendor: 'nvidia',
    title: 'GeForce Game Ready / Studio',
    description: 'Выбери точную модель видеокарты и ветку драйвера на странице NVIDIA.',
    url: 'https://www.nvidia.com/en-us/drivers/',
  },
  {
    id: 'nvidia-cuda',
    vendor: 'nvidia',
    title: 'CUDA Toolkit',
    description:
      'Для приложений и игр, которым нужны вычисления на видеокарте. Сверь версию с требованиями программы.',
    url: 'https://developer.nvidia.com/cuda-downloads',
  },
  {
    id: 'amd-software',
    vendor: 'amd',
    title: 'AMD Software · Adrenalin',
    description:
      'Официальный установщик драйвера Radeon. Версию и совместимость проверяет сам производитель.',
    url: 'https://www.amd.com/en/support/download/drivers.html',
  },
  {
    id: 'amd-chipset',
    vendor: 'amd',
    title: 'AMD · чипсет',
    description:
      'Выбери сокет и модель платы на сайте AMD. Драйвер процессора отдельно обычно не требуется.',
    url: 'https://www.amd.com/en/support/download/drivers.html',
  },
  {
    id: 'intel-dsa',
    vendor: 'intel',
    title: 'Intel · автоопределение',
    description:
      'Официальный помощник Intel подбирает поддерживаемые драйверы для устройств Intel.',
    url: 'https://www.intel.com/content/www/us/en/support/detect.html',
  },
  {
    id: 'intel-chipset',
    vendor: 'intel',
    title: 'Intel · чипсет',
    description:
      'Официальные пакеты чипсета. Совместимость зависит от модели платы и поколения процессора.',
    url: 'https://www.intel.com/content/www/us/en/download/19347/chipset-inf-utility.html',
  },
  {
    id: 'intel-wifi',
    vendor: 'bluetooth',
    title: 'Intel · Wi-Fi',
    description: 'Пакет для поддерживаемых беспроводных адаптеров Intel под Windows 10 и 11.',
    url: 'https://www.intel.com/content/www/us/en/download/19351/intel-wireless-wi-fi-drivers-for-windows-10-and-windows-11.html',
  },
  {
    id: 'intel-bluetooth',
    vendor: 'bluetooth',
    title: 'Intel · Bluetooth',
    description: 'Выбери пакет под свой адаптер и версию Windows.',
    url: 'https://www.intel.com/content/www/us/en/download/18649/intel-wireless-bluetooth-drivers-for-windows-10-and-windows-11.html',
  },
  {
    id: 'realtek-audio',
    vendor: 'realtek',
    title: 'Realtek · звук',
    description:
      'Каталог звуковых драйверов. Часто надёжнее взять пакет у производителя платы или ноутбука.',
    url: 'https://www.realtek.com/Download/List?cate_id=593',
  },
  {
    id: 'realtek-network',
    vendor: 'realtek',
    title: 'Realtek · сеть',
    description: 'Драйверы проводной сети Ethernet. Подбирай по модели контроллера.',
    url: 'https://www.realtek.com/Download/List?cate_id=584',
  },
  {
    id: 'windows-update',
    vendor: 'usb',
    title: 'USB / контроллеры',
    description:
      'Контроллеры USB обычно ставит сам Windows. Если что-то не работает, ищи пакет у производителя платы.',
    url: 'https://www.catalog.update.microsoft.com/',
  },
];

/** Какие производители найдены в компьютере; без сведений о железе показываем основные три. */
export function detectVendors(hardware: HardwareOverview | null): VendorId[] {
  const found: VendorId[] = [];
  const has = (text: string | null | undefined, word: string) =>
    (text ?? '').toLowerCase().includes(word);
  if (hardware) {
    for (const gpu of hardware.gpus) {
      for (const [word, id] of [
        ['nvidia', 'nvidia'],
        ['amd', 'amd'],
        ['radeon', 'amd'],
        ['intel', 'intel'],
      ] as const) {
        if ((has(gpu.vendor, word) || has(gpu.model, word)) && !found.includes(id)) {
          found.push(id);
        }
      }
    }
    for (const [word, id] of [
      ['intel', 'intel'],
      ['amd', 'amd'],
      ['ryzen', 'amd'],
    ] as const) {
      if (has(hardware.cpuBrand, word) && !found.includes(id)) {
        found.push(id);
      }
    }
  }
  if (found.length === 0) {
    found.push('nvidia', 'amd', 'intel');
  }
  return [...found, 'realtek', 'bluetooth', 'usb'];
}
