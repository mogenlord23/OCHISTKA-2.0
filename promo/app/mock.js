// Подмена Tauri-бэкенда для записи интерфейса в браузере. Данные демонстрационные.
(() => {
  const GB = 1024 ** 3;
  const t0 = performance.now();
  const wave = (a, b, p) => a + (b - a) * (0.5 + 0.5 * Math.sin(((performance.now() - t0) / 1000) * p));
  const overview = () => {
    const cpu = wave(22, 63, 1.7) + wave(0, 8, 5.3) - 4;
    const ram = wave(52, 61, 0.6);
    return {
      cpu: { usagePercent: cpu, logicalCores: 12 },
      memory: { totalBytes: 32 * GB, usedBytes: (ram / 100) * 32 * GB, usagePercent: ram },
      disks: [
        { displayName: 'Системный', mountPoint: 'C:\\', totalBytes: 512 * GB, usedBytes: 318 * GB, usagePercent: 62.1 },
        { displayName: 'Данные', mountPoint: 'D:\\', totalBytes: 1024 * GB, usedBytes: 402 * GB, usagePercent: 39.3 },
      ],
    };
  };
  const finds = [
    { path: 'C:\\Users\\User\\Downloads\\invoice.pdf.exe', signature: 'Win.Trojan.Agent' },
    { path: 'C:\\Users\\User\\Downloads\\crack_setup.exe', signature: 'Win.Malware.Generic' },
    { path: 'C:\\Users\\User\\Downloads\\update_tool.scr', signature: 'Win.Adware.Gen' },
  ];
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  window.__mockLog = [];
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
    transformCallback: () => 0,
    unregisterCallback: () => {},
    convertFileSrc: (s) => s,
    invoke: async (cmd, args) => {
      window.__mockLog.push(cmd);
      switch (cmd) {
        case 'get_system_overview': return overview();
        case 'clamav_status': return { available: true, reason: '' };
        case 'default_scan_target': return 'C:\\Users\\User\\Downloads';
        case 'scan_start': await wait(window.__scanMs ?? 5000); return { scannedFiles: 18432, infectedFiles: 3, errors: 0, findings: finds };
        case 'scan_cancel': return true;
        case 'assess_file': return { score: 80, reasons: ['двойное расширение', 'исполняемый файл в загрузках'] };
        default: return null;
      }
    },
  };
})();
