//! Сеть только для чтения. Адаптеры — sysinfo `Networks`, TCP-соединения — безопасный крейт
//! netstat2 (собственного unsafe здесь нет). Код собирается на всех ОС; порт текущей ОС
//! выбирает `current_network_port`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpSocketInfo,
};
use ochistka_core::network::{
    bytes_per_second, format_address, format_mac, NetworkAdapter, NetworkPort, TcpConnection,
};
use ochistka_core::system::CoreError;
use sysinfo::{Networks, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// Пауза первого замера: скорость считается между двумя обновлениями.
const FIRST_SAMPLE_PAUSE: Duration = Duration::from_millis(250);

struct Sampler {
    networks: Networks,
    refreshed_at: Instant,
}

pub struct WindowsNetwork {
    sampler: Mutex<Option<Sampler>>,
}

impl WindowsNetwork {
    /// Ничего не читает: первый замер выполняется при первом вызове `adapters()`.
    pub fn new() -> Self {
        Self {
            sampler: Mutex::new(None),
        }
    }
}

impl Default for WindowsNetwork {
    fn default() -> Self {
        Self::new()
    }
}

fn probe(message: impl ToString) -> CoreError {
    CoreError::Probe(message.to_string())
}

impl NetworkPort for WindowsNetwork {
    fn adapters(&self) -> Result<Vec<NetworkAdapter>, CoreError> {
        // Отравление мьютекса не фатально: каждый вызов заново обновляет снимок.
        let mut guard = self
            .sampler
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_none() {
            let networks = Networks::new_with_refreshed_list().map_err(probe)?;
            std::thread::sleep(FIRST_SAMPLE_PAUSE);
            *guard = Some(Sampler {
                networks,
                refreshed_at: Instant::now(),
            });
        }
        let sampler = guard
            .as_mut()
            .ok_or_else(|| probe("снимок сети не создан"))?;
        let elapsed_ms = u64::try_from(sampler.refreshed_at.elapsed().as_millis()).unwrap_or(0);
        sampler.networks.refresh(true);
        sampler.refreshed_at = Instant::now();

        let mut list: Vec<NetworkAdapter> = sampler
            .networks
            .iter()
            .map(|(name, data)| NetworkAdapter {
                name: name.to_string_lossy().into_owned(),
                mac: format_mac(data.mac_address().0),
                received_bytes: data.total_received(),
                transmitted_bytes: data.total_transmitted(),
                receive_rate: bytes_per_second(data.received(), elapsed_ms),
                transmit_rate: bytes_per_second(data.transmitted(), elapsed_ms),
                addresses: data
                    .ip_networks()
                    .iter()
                    .map(|net| format_address(&net.addr.to_string(), net.prefix))
                    .collect(),
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    fn connections(&self) -> Result<Vec<TcpConnection>, CoreError> {
        let sockets = get_sockets_info(
            AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
            ProtocolFlags::TCP,
        )
        .map_err(probe)?;

        let mut system = System::new().map_err(probe)?;
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );

        let list = sockets
            .into_iter()
            .filter_map(|socket| {
                let pid = socket.associated_pids.first().copied();
                match socket.protocol_socket_info {
                    ProtocolSocketInfo::Tcp(tcp) => Some(to_connection(&tcp, pid, &system)),
                    ProtocolSocketInfo::Udp(_) => None,
                }
            })
            .collect();
        Ok(list)
    }
}

fn to_connection(tcp: &TcpSocketInfo, pid: Option<u32>, system: &System) -> TcpConnection {
    TcpConnection {
        local_addr: tcp.local_addr.to_string(),
        local_port: tcp.local_port,
        remote_addr: tcp.remote_addr.to_string(),
        remote_port: tcp.remote_port,
        state: tcp.state.to_string(),
        pid,
        process_name: pid
            .and_then(|pid| system.process(Pid::from_u32(pid)))
            .map(|process| process.name().to_string_lossy().into_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn adapters_do_not_fail() {
        // В контейнере адаптеров может не быть; важно, что чтение не падает.
        let list = WindowsNetwork::new().adapters();
        assert!(list.is_ok(), "{list:?}");
    }

    #[test]
    fn finds_own_listening_socket_with_pid() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let list = WindowsNetwork::new().connections().unwrap();
        let mine = list
            .iter()
            .find(|c| c.local_port == port && c.local_addr == "127.0.0.1")
            .expect("own socket listed");
        assert_eq!(mine.state, "LISTEN");
        assert_eq!(mine.pid, Some(std::process::id()));
        assert!(mine.process_name.is_some());
    }
}
