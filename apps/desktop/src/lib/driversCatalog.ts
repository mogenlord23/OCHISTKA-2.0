/** Каталог драйверов по производителям. Данные приходят из бэкенда (`drivers_catalog`); здесь
 * только описание плиток производителей, фильтр и определение «твоего» оборудования. */

import type { HardwareOverview } from './types';

export type VendorId = 'nvidia' | 'amd' | 'intel' | 'realtek' | 'bluetooth' | 'usb' | 'oem';

export interface Vendor {
  id: VendorId;
  label: string;
  /** Цвет плашки. */
  color: string;
  /** Буквенная метка для производителей без знака в simple-icons. */
  mark: string | null;
}

export const VENDORS: Record<VendorId, Vendor> = {
  nvidia: { id: 'nvidia', label: 'NVIDIA', color: '#76b900', mark: null },
  amd: { id: 'amd', label: 'AMD', color: '#ed1c24', mark: null },
  intel: { id: 'intel', label: 'Intel', color: '#0071c5', mark: null },
  realtek: { id: 'realtek', label: 'Realtek', color: '#0093d0', mark: 'R' },
  bluetooth: { id: 'bluetooth', label: 'Wi-Fi / Bluetooth', color: '#0a82ff', mark: null },
  usb: { id: 'usb', label: 'USB', color: '#ff00d4', mark: 'USB' },
  oem: { id: 'oem', label: 'Производители ПК', color: '#ff7a00', mark: null },
};

export const VENDOR_ORDER: readonly VendorId[] = [
  'nvidia',
  'amd',
  'intel',
  'realtek',
  'bluetooth',
  'usb',
  'oem',
];

export type DriverAction = 'install' | 'openSite';

/** Пункт каталога из бэкенда. Адресов здесь нет: их знает только ядро. */
export interface DriverItem {
  id: string;
  vendor: VendorId;
  title: string;
  description: string;
  action: DriverAction;
  windowsUpdate: boolean;
}

/** Пункты одного производителя; `null` — все. */
export function filterByVendor(
  items: readonly DriverItem[],
  vendor: VendorId | null,
): DriverItem[] {
  return vendor === null ? [...items] : items.filter((item) => item.vendor === vendor);
}

export interface VendorLayout {
  /** Производители в порядке показа: найденные в компьютере идут первыми. */
  ordered: VendorId[];
  /** Найденные в компьютере (метка «ваше устройство»). */
  detected: ReadonlySet<VendorId>;
}

const OEM_WORDS = ['asus', 'msi', 'micro-star', 'gigabyte', 'lenovo', 'dell', 'hp', 'acer'];

/** Какие производители найдены по видеокартам, процессору и плате. */
export function detectVendors(hardware: HardwareOverview | null): ReadonlySet<VendorId> {
  const found = new Set<VendorId>();
  if (!hardware) {
    return found;
  }
  const has = (text: string | null | undefined, word: string) =>
    (text ?? '').toLowerCase().includes(word);
  for (const gpu of hardware.gpus) {
    for (const [word, id] of [
      ['nvidia', 'nvidia'],
      ['geforce', 'nvidia'],
      ['amd', 'amd'],
      ['radeon', 'amd'],
      ['intel', 'intel'],
    ] as const) {
      if (has(gpu.vendor, word) || has(gpu.model, word)) {
        found.add(id);
      }
    }
  }
  for (const [word, id] of [
    ['intel', 'intel'],
    ['amd', 'amd'],
    ['ryzen', 'amd'],
  ] as const) {
    if (has(hardware.cpuBrand, word)) {
      found.add(id);
    }
  }
  const board = hardware.motherboard.vendor?.toLowerCase().split(/[\s,.]+/) ?? [];
  if (OEM_WORDS.some((word) => board.includes(word) || board.join(' ').includes(word))) {
    found.add('oem');
  }
  return found;
}

export function layoutVendors(hardware: HardwareOverview | null): VendorLayout {
  const detected = detectVendors(hardware);
  const ordered = [
    ...VENDOR_ORDER.filter((id) => detected.has(id)),
    ...VENDOR_ORDER.filter((id) => !detected.has(id)),
  ];
  return { ordered, detected };
}
