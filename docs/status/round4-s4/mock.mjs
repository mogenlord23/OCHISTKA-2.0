// Общий подставной бэкенд для скриншотов S4.
export const mock = `(() => {
  const GB = 1073741824; let n = 0; window.__calls = [];
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd, a) => {
    window.__calls.push([cmd, a]);
    if (cmd === 'get_system_overview') { n++; return { cpu:{ usagePercent: 30 + (n%7), logicalCores: 8 }, memory:{ totalBytes: 24*GB, usedBytes: 15*GB, usagePercent: 65 }, disks:[{displayName:'Windows (C:)', mountPoint:'C:\\\\', totalBytes:111*GB, usedBytes:60*GB, usagePercent:54, kind:'ssd'}], osName:'Майкрософт Windows 11 Домашняя', osVersion:'10.0.26200' }; }
    if (cmd === 'get_hardware') { await new Promise(r=>setTimeout(r, 600)); return { gpus:[{ vendor:'NVIDIA', model:'GeForce GTX 1060 3GB', usagePercent: 5, totalMemoryBytes:3*GB, usedMemoryBytes:GB }], motherboard:{ vendor:'ASRock', name:'Z170 Extreme4', version:null }, cpuBrand:'Intel(R) Core(TM) i7-6700 CPU @ 3.40GHz', driveModels:['KINGSTON SHFS37A120G'] }; }
    if (cmd === 'drivers_catalog') return [
      { id:'nvidia-app', vendor:'nvidia', maker:'NVIDIA', device:'Видеокарта', title:'NVIDIA App · драйверы', description:'Официальное приложение NVIDIA.', action:'install', windowsUpdate:false },
      { id:'realtek-audio', vendor:'audio', maker:'Realtek', device:'Звук', title:'Realtek · звук', description:'Каталог звуковых драйверов.', action:'openSite', windowsUpdate:true },
      { id:'realtek-network', vendor:'audio', maker:'Realtek', device:'Сеть: Ethernet и Wi-Fi', title:'Realtek · Ethernet и Wi-Fi', description:'Каталог сетевых контроллеров Realtek.', action:'openSite', windowsUpdate:true },
      { id:'intel-wifi', vendor:'audio', maker:'Intel', device:'Wi-Fi', title:'Intel · Wi-Fi', description:'Пакет для адаптеров Intel.', action:'install', windowsUpdate:true },
      { id:'intel-bluetooth', vendor:'audio', maker:'Intel', device:'Bluetooth', title:'Intel · Bluetooth', description:'Выберите пакет под свой адаптер.', action:'openSite', windowsUpdate:true },
      { id:'audio-other', vendor:'audio', maker:'Другие производители', device:'Звук', title:'Другие звуковые устройства', description:'Сначала Windows Update.', action:'openSite', windowsUpdate:true },
    ];
    if (cmd.startsWith('plugin:')) return null;
    return {};
  } };
})();`;
