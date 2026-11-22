# Конфигурация ядра для «Только сайты»

Это то, что приложение передаёт ядру (sing-box) в области «Только сайты». Образец построен на вымышленном сервере, адрес сервера и ключи заменены метками `<адрес сервера>` и `<ключ>`. Список сайтов в образце: `youtube.com`, `discord.gg` (у владельца берётся список из панели «Сайты»). Создан командой `cargo run -p ochistka-core --example dump_vpn_config -- <папка>`; формат проверяет `sing-box check` на CI (workflow `vpn-config-check`).

Как читать:
1. `hijack-dns` — запросы имён из туннеля обрабатывает ядро.
2. Адрес сервера — напрямую (иначе соединение с сервером попало бы в туннель).
3. Локальные адреса (`ip_is_private`) — напрямую.
4. Домены из списка и все их поддомены (`domain_suffix`) — через VPN (`proxy`).
5. Всё остальное — напрямую (`route.final = direct`).
6. Имена сайтов из списка разрешаются через VPN (`remote`), остальные — локально (`dns.final = local`).
7. Системный прокси Windows не используется; весь трафик перехватывает туннельный адаптер.

```json
{
  "dns": {
    "cache_capacity": 4096,
    "final": "local",
    "rules": [
      {
        "domain": [
          "<адрес сервера>"
        ],
        "server": "local"
      },
      {
        "domain_suffix": [
          "youtube.com",
          "discord.gg"
        ],
        "server": "remote"
      }
    ],
    "servers": [
      {
        "tag": "local",
        "type": "local"
      },
      {
        "detour": "proxy",
        "server": "1.1.1.1",
        "tag": "remote",
        "type": "https"
      }
    ]
  },
  "inbounds": [
    {
      "listen": "127.0.0.1",
      "listen_port": 20808,
      "tag": "mixed-in",
      "type": "mixed"
    },
    {
      "address": [
        "172.19.0.1/30",
        "fdfe:dcba:9876::1/126"
      ],
      "auto_route": true,
      "mtu": 1400,
      "stack": "mixed",
      "strict_route": true,
      "tag": "tun-in",
      "type": "tun"
    }
  ],
  "log": {
    "level": "warn",
    "timestamp": true
  },
  "outbounds": [
    {
      "password": "<ключ>",
      "server": "<адрес сервера>",
      "server_port": 443,
      "tag": "proxy",
      "tls": {
        "enabled": true,
        "server_name": "<адрес сервера>"
      },
      "type": "trojan"
    },
    {
      "tag": "direct",
      "type": "direct"
    }
  ],
  "route": {
    "auto_detect_interface": true,
    "default_domain_resolver": "local",
    "final": "direct",
    "rules": [
      {
        "action": "hijack-dns",
        "protocol": "dns"
      },
      {
        "domain": [
          "<адрес сервера>"
        ],
        "outbound": "direct"
      },
      {
        "ip_is_private": true,
        "outbound": "direct"
      },
      {
        "domain_suffix": [
          "youtube.com",
          "discord.gg"
        ],
        "outbound": "proxy"
      }
    ]
  }
}
```
