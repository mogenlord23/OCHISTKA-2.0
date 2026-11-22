//! Локальный прокси на 127.0.0.1: HTTP CONNECT, обычный HTTP через прокси и SOCKS5 (без пароля).
//! Для TLS-соединений на порт 443 к сайтам из списка первый пакет клиента (ClientHello) уходит
//! двумя частями, разрез внутри имени сайта. Остальной трафик копируется без изменений.
//!
//! Потоки: один принимающий и по одному на соединение. После `stop()` не остаётся ни одного.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::sites::SiteList;
use crate::tls;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const FIRST_PACKET_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_HEADER: usize = 16 * 1024;
/// Короткая пауза между двумя частями ClientHello: так они уходят отдельными пакетами.
const SPLIT_PAUSE: Duration = Duration::from_millis(5);

struct Shared {
    stop: AtomicBool,
    /// Только для тестов: принимающий поток «падает».
    #[cfg(test)]
    crash: AtomicBool,
    live_threads: AtomicUsize,
    next_id: AtomicU64,
    sockets: Mutex<Vec<(u64, TcpStream)>>,
    handles: Mutex<Vec<JoinHandle<()>>>,
    sites: SiteList,
}

/// Считает живые потоки: увеличивает при создании, уменьшает при выходе потока.
struct ThreadGuard(Arc<Shared>);

impl ThreadGuard {
    fn new(shared: &Arc<Shared>) -> Self {
        shared.live_threads.fetch_add(1, Ordering::SeqCst);
        Self(shared.clone())
    }
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        self.0.live_threads.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct ProxyServer {
    addr: SocketAddr,
    shared: Arc<Shared>,
    accept: Option<JoinHandle<()>>,
}

impl ProxyServer {
    /// Запускает прокси на свободном порту 127.0.0.1.
    pub fn start(sites: SiteList) -> io::Result<Self> {
        Self::start_with_exit_hook(sites, None)
    }

    /// То же, но `on_dead` вызывается в принимающем потоке, если он остановился НЕ по `stop()`
    /// (ошибка приёма или падение). Так обход узнаёт о гибели прокси без опроса и таймеров.
    ///
    /// Порт выбирает система: слушаем порт 0, берём назначенный и сразу используем его (тот же
    /// сокет, поэтому порт гарантированно наш и свободен).
    pub fn start_with_exit_hook(
        sites: SiteList,
        on_dead: Option<Box<dyn FnOnce() + Send + 'static>>,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let addr = listener.local_addr()?;
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            #[cfg(test)]
            crash: AtomicBool::new(false),
            live_threads: AtomicUsize::new(0),
            next_id: AtomicU64::new(0),
            sockets: Mutex::new(Vec::new()),
            handles: Mutex::new(Vec::new()),
            sites,
        });
        let guard = ThreadGuard::new(&shared);
        let for_accept = shared.clone();
        let accept = thread::Builder::new()
            .name("proxy-bypass-accept".into())
            .spawn(move || {
                let _guard = guard;
                let shared = for_accept.clone();
                let ended = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    accept_loop(listener, for_accept)
                }));
                // Остановка без запроса `stop()` (ошибка или падение) — сообщаем владельцу.
                if ended.is_err() || !shared.stop.load(Ordering::SeqCst) {
                    if let Some(hook) = on_dead {
                        hook();
                    }
                }
            })?;
        Ok(Self {
            addr,
            shared,
            accept: Some(accept),
        })
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Принимающий поток ещё работает.
    pub fn is_alive(&self) -> bool {
        self.accept.as_ref().is_some_and(|h| !h.is_finished())
    }

    /// Проверка: к 127.0.0.1:порт можно подключиться (сервер действительно принимает).
    pub fn probe(&self) -> bool {
        TcpStream::connect_timeout(&self.addr, Duration::from_secs(2)).is_ok()
    }

    /// Только для тестов: принимающий поток падает при следующем подключении.
    #[cfg(test)]
    pub(crate) fn crash_for_test(&self) {
        self.shared.crash.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_secs(2));
    }

    /// Сколько потоков прокси сейчас живо (после `stop()` — ноль).
    pub fn live_threads(&self) -> usize {
        self.shared.live_threads.load(Ordering::SeqCst)
    }

    /// Останавливает прокси: закрывает приём, рвёт соединения и ждёт завершения всех потоков.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let Some(accept) = self.accept.take() else {
            return;
        };
        self.shared.stop.store(true, Ordering::SeqCst);
        // Будим принимающий поток: он ждёт соединения.
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_secs(2));
        let _ = accept.join();
        if let Ok(sockets) = self.shared.sockets.lock() {
            for (_, s) in sockets.iter() {
                let _ = s.shutdown(Shutdown::Both);
            }
        }
        loop {
            let handle = self.shared.handles.lock().ok().and_then(|mut h| h.pop());
            match handle {
                Some(h) => {
                    let _ = h.join();
                }
                None => break,
            }
        }
    }
}

impl Drop for ProxyServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn accept_loop(listener: TcpListener, shared: Arc<Shared>) {
    for incoming in listener.incoming() {
        if shared.stop.load(Ordering::SeqCst) {
            break;
        }
        #[cfg(test)]
        if shared.crash.load(Ordering::SeqCst) {
            panic!("тестовое падение принимающего потока");
        }
        let client = match incoming {
            Ok(client) => client,
            // Разовые сбои отдельного подключения не останавливают приём.
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::Interrupted
                        | io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            // Остальное — слушающий сокет сломан: поток завершается, владелец узнает.
            Err(_) => break,
        };
        let guard = ThreadGuard::new(&shared);
        let for_conn = shared.clone();
        let spawned = thread::Builder::new()
            .name("proxy-bypass-conn".into())
            .spawn(move || {
                let _guard = guard;
                let _ = handle(&for_conn, client);
            });
        if let (Ok(handle), Ok(mut handles)) = (spawned, shared.handles.lock()) {
            handles.retain(|h| !h.is_finished());
            handles.push(handle);
        }
    }
}

/// Регистрирует сокет, чтобы `stop()` мог его закрыть; снимает регистрацию при выходе.
struct Registered<'a> {
    shared: &'a Shared,
    id: u64,
}

impl<'a> Registered<'a> {
    fn new(shared: &'a Shared, stream: &TcpStream) -> Option<Self> {
        let clone = stream.try_clone().ok()?;
        let id = shared.next_id.fetch_add(1, Ordering::SeqCst);
        shared.sockets.lock().ok()?.push((id, clone));
        Some(Self { shared, id })
    }
}

impl Drop for Registered<'_> {
    fn drop(&mut self) {
        if let Ok(mut s) = self.shared.sockets.lock() {
            s.retain(|(id, _)| *id != self.id);
        }
    }
}

enum Request {
    /// Ответ клиенту уже нужен (CONNECT / SOCKS5): после соединения.
    Tunnel {
        host: String,
        port: u16,
        socks: bool,
    },
    /// Обычный HTTP через прокси: запрос пересылается как есть.
    Forward {
        host: String,
        port: u16,
        buffered: Vec<u8>,
    },
}

fn handle(shared: &Arc<Shared>, client: TcpStream) -> io::Result<()> {
    client.set_nodelay(true)?;
    client.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
    client.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
    let _client_reg = Registered::new(shared, &client);
    let mut first = [0u8; 1];
    if client.peek(&mut first)? == 0 {
        return Ok(());
    }
    let request = if first[0] == 5 {
        read_socks5(&client)?
    } else {
        read_http(&client)?
    };
    let (host, port, socks, buffered) = match request {
        Request::Tunnel { host, port, socks } => (host, port, socks, None),
        Request::Forward {
            host,
            port,
            buffered,
        } => (host, port, false, Some(buffered)),
    };
    let upstream = match connect_upstream(&host, port) {
        Ok(s) => s,
        Err(e) => {
            let _ = reply_failure(&client, socks);
            return Err(e);
        }
    };
    let _upstream_reg = Registered::new(shared, &upstream);
    match buffered {
        Some(bytes) => (&upstream).write_all(&bytes)?,
        None => reply_success(&client, socks)?,
    }
    if port == 443 && shared.sites.matches(&host) {
        forward_first_packet(&client, &upstream)?;
    }
    client.set_read_timeout(None)?;
    client.set_write_timeout(None)?;
    pipe(&client, &upstream);
    Ok(())
}

/// Первый пакет клиента (ClientHello) к сайту из списка уходит двумя частями.
fn forward_first_packet(client: &TcpStream, upstream: &TcpStream) -> io::Result<()> {
    client.set_read_timeout(Some(FIRST_PACKET_TIMEOUT))?;
    let mut buf = vec![0u8; MAX_HEADER];
    let n = match (&*client).read(&mut buf) {
        Ok(n) => n,
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            0
        }
        Err(e) => return Err(e),
    };
    buf.truncate(n);
    if buf.is_empty() {
        return Ok(());
    }
    let mut up = upstream;
    match tls::split_point(&buf) {
        Some(k) => {
            up.write_all(&buf[..k])?;
            up.flush()?;
            thread::sleep(SPLIT_PAUSE);
            up.write_all(&buf[k..])?;
        }
        None => up.write_all(&buf)?,
    }
    up.flush()
}

fn pipe(client: &TcpStream, upstream: &TcpStream) {
    thread::scope(|scope| {
        let _ = thread::Builder::new()
            .name("proxy-bypass-up".into())
            .spawn_scoped(scope, || copy_one_way(client, upstream));
        copy_one_way(upstream, client);
    });
    let _ = client.shutdown(Shutdown::Both);
    let _ = upstream.shutdown(Shutdown::Both);
}

fn copy_one_way(from: &TcpStream, to: &TcpStream) {
    let (mut r, mut w) = (from, to);
    let ok = io::copy(&mut r, &mut w).is_ok();
    let _ = to.shutdown(Shutdown::Write);
    if !ok {
        let _ = from.shutdown(Shutdown::Both);
    }
}

fn connect_upstream(host: &str, port: u16) -> io::Result<TcpStream> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "нет адреса");
    for addr in (host, port).to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(s) => {
                s.set_nodelay(true)?;
                return Ok(s);
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn reply_success(client: &TcpStream, socks: bool) -> io::Result<()> {
    let mut c = client;
    if socks {
        c.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0])
    } else {
        c.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
    }
}

fn reply_failure(client: &TcpStream, socks: bool) -> io::Result<()> {
    let mut c = client;
    if socks {
        c.write_all(&[5, 4, 0, 1, 0, 0, 0, 0, 0, 0])
    } else {
        c.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n")
    }
}

fn read_exact_n(client: &TcpStream, n: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    (&*client).read_exact(&mut buf)?;
    Ok(buf)
}

/// SOCKS5 без пароля, команда CONNECT; адреса IPv4, IPv6 и имя.
fn read_socks5(client: &TcpStream) -> io::Result<Request> {
    let bad = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_string());
    let head = read_exact_n(client, 2)?;
    if head[0] != 5 {
        return Err(bad("не SOCKS5"));
    }
    let methods = read_exact_n(client, head[1] as usize)?;
    let mut c = client;
    if !methods.contains(&0) {
        c.write_all(&[5, 0xFF])?;
        return Err(bad("нет метода без пароля"));
    }
    c.write_all(&[5, 0])?;
    let req = read_exact_n(client, 4)?;
    if req[0] != 5 || req[1] != 1 {
        c.write_all(&[5, 7, 0, 1, 0, 0, 0, 0, 0, 0])?;
        return Err(bad("только CONNECT"));
    }
    let host = match req[3] {
        1 => {
            let a = read_exact_n(client, 4)?;
            std::net::Ipv4Addr::new(a[0], a[1], a[2], a[3]).to_string()
        }
        3 => {
            let len = read_exact_n(client, 1)?[0] as usize;
            String::from_utf8(read_exact_n(client, len)?).map_err(|_| bad("имя не UTF-8"))?
        }
        4 => {
            let a: [u8; 16] = read_exact_n(client, 16)?
                .try_into()
                .map_err(|_| bad("адрес"))?;
            std::net::Ipv6Addr::from(a).to_string()
        }
        _ => {
            c.write_all(&[5, 8, 0, 1, 0, 0, 0, 0, 0, 0])?;
            return Err(bad("тип адреса"));
        }
    };
    let p = read_exact_n(client, 2)?;
    Ok(Request::Tunnel {
        host,
        port: u16::from_be_bytes([p[0], p[1]]),
        socks: true,
    })
}

fn read_http(client: &TcpStream) -> io::Result<Request> {
    let bad = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_string());
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > MAX_HEADER {
            return Err(bad("заголовок слишком длинный"));
        }
        let n = (&*client).read(&mut chunk)?;
        if n == 0 {
            return Err(bad("соединение закрыто"));
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let line = head.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = split_host_port(target, 443).ok_or_else(|| bad("адрес"))?;
        return Ok(Request::Tunnel {
            host,
            port,
            socks: false,
        });
    }
    // Обычный запрос через прокси: адрес целиком http://host[:port]/путь.
    let rest = target
        .strip_prefix("http://")
        .ok_or_else(|| bad("нужен абсолютный адрес"))?;
    let authority = rest.split(['/', '?']).next().unwrap_or("");
    let (host, port) = split_host_port(authority, 80).ok_or_else(|| bad("адрес"))?;
    Ok(Request::Forward {
        host,
        port,
        buffered: buf,
    })
}

/// `host`, `host:port`, `[v6]:port`.
fn split_host_port(text: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = text.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        let port = match tail.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None => default_port,
        };
        return Some((host.to_string(), port));
    }
    match text.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => Some((host.to_string(), port.parse().ok()?)),
        _ if !text.is_empty() => Some((text.to_string(), default_port)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Сервер-«сайт»: принимает одно соединение и отдаёт то, что получил, частями (по чтениям).
    fn echo_site() -> (u16, JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_millis(400)))
                .unwrap();
            let mut parts = Vec::new();
            let mut buf = [0u8; 4096];
            while let Ok(n) = s.read(&mut buf) {
                if n == 0 {
                    break;
                }
                parts.push(buf[..n].to_vec());
                let _ = s.write_all(b"pong");
            }
            parts
        });
        (port, handle)
    }

    fn read_until(s: &mut TcpStream, needle: &[u8]) -> Vec<u8> {
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut got = Vec::new();
        let mut b = [0u8; 256];
        while !got.windows(needle.len()).any(|w| w == needle) {
            let n = s.read(&mut b).unwrap();
            assert!(n > 0, "закрыто: {got:?}");
            got.extend_from_slice(&b[..n]);
        }
        got
    }

    #[test]
    fn http_connect_to_a_site_outside_the_list_passes_unchanged() {
        let (site_port, site) = echo_site();
        let server = ProxyServer::start(SiteList::new(["youtube.com"])).unwrap();
        let mut c = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        write!(
            c,
            "CONNECT 127.0.0.1:{site_port} HTTP/1.1\r\nHost: x\r\n\r\n"
        )
        .unwrap();
        read_until(&mut c, b"200 Connection Established\r\n\r\n");
        c.write_all(b"hello").unwrap();
        read_until(&mut c, b"pong");
        drop(c);
        let parts = site.join().unwrap();
        assert_eq!(parts.concat(), b"hello");
        server.stop();
    }

    #[test]
    fn socks5_connect_works_with_an_ipv4_target() {
        let (site_port, site) = echo_site();
        let server = ProxyServer::start(SiteList::default()).unwrap();
        let mut c = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        c.write_all(&[5, 1, 0]).unwrap();
        assert_eq!(read_until(&mut c, &[5, 0]), [5, 0]);
        let mut req = vec![5, 1, 0, 1, 127, 0, 0, 1];
        req.extend(site_port.to_be_bytes());
        c.write_all(&req).unwrap();
        let reply = read_until(&mut c, &[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&reply[..2], [5, 0]);
        c.write_all(b"ping").unwrap();
        read_until(&mut c, b"pong");
        drop(c);
        assert_eq!(site.join().unwrap().concat(), b"ping");
        server.stop();
    }

    #[test]
    fn socks5_with_a_domain_name_and_a_refused_target() {
        let server = ProxyServer::start(SiteList::default()).unwrap();
        let mut c = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        c.write_all(&[5, 1, 0]).unwrap();
        read_until(&mut c, &[5, 0]);
        // Порт без слушателя: соединение отклонено, ответ «ошибка», а не зависание.
        let dead = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let dead_port = dead.local_addr().unwrap().port();
        drop(dead);
        let mut req = vec![5, 1, 0, 3, 9];
        req.extend(b"localhost");
        req.extend(dead_port.to_be_bytes());
        c.write_all(&req).unwrap();
        let reply = read_until(&mut c, &[0]);
        assert_eq!(reply[0], 5);
        assert_ne!(reply[1], 0, "ошибка соединения должна быть сообщена");
        server.stop();
    }

    #[test]
    fn plain_http_through_the_proxy_is_forwarded_as_is() {
        let (site_port, site) = echo_site();
        let server = ProxyServer::start(SiteList::default()).unwrap();
        let mut c = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        let request = format!("GET http://127.0.0.1:{site_port}/a HTTP/1.1\r\nHost: x\r\n\r\n");
        c.write_all(request.as_bytes()).unwrap();
        read_until(&mut c, b"pong");
        drop(c);
        assert_eq!(site.join().unwrap().concat(), request.as_bytes());
        server.stop();
    }

    #[test]
    fn a_client_hello_to_a_listed_site_is_cut_inside_the_name_and_arrives_whole() {
        // «Сайт» слушает 443 недоступен, поэтому проверяем резку напрямую на порту сайта через
        // тестовый вход: список содержит loopback, порт 443 подменяется отдельной функцией.
        let hello = tls::client_hello("youtube.com");
        let (site_port, site) = echo_site();
        let a = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let b = TcpStream::connect(("127.0.0.1", site_port)).unwrap();
        let a_port = a.local_addr().unwrap().port();
        let mut sender = TcpStream::connect(("127.0.0.1", a_port)).unwrap();
        let (receiver, _) = a.accept().unwrap();
        sender.write_all(&hello).unwrap();
        forward_first_packet(&receiver, &b).unwrap();
        drop(b);
        let parts = site.join().unwrap();
        assert!(
            parts.len() >= 2,
            "первый пакет должен прийти двумя частями: {}",
            parts.len()
        );
        let joined = parts.concat();
        assert_eq!(joined, hello, "данные не потеряны и не изменены");
        let k = tls::split_point(&hello).unwrap();
        assert_eq!(parts[0].len(), k);
        let whole = |p: &[u8]| p.windows(11).any(|w| w == b"youtube.com");
        assert!(parts.iter().all(|p| !whole(p)), "имя целиком в одной части");
    }

    #[test]
    fn stop_leaves_no_threads_even_with_open_connections() {
        let (site_port, _site) = echo_site();
        let server = ProxyServer::start(SiteList::default()).unwrap();
        assert!(server.live_threads() >= 1);
        let mut c = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        write!(c, "CONNECT 127.0.0.1:{site_port} HTTP/1.1\r\n\r\n").unwrap();
        read_until(&mut c, b"Established\r\n\r\n");
        let shared = server.shared.clone();
        server.stop();
        assert_eq!(shared.live_threads.load(Ordering::SeqCst), 0);
        // Прокси закрыт: новое соединение не принимается.
        let mut buf = [0u8; 1];
        c.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        assert!(matches!(c.read(&mut buf), Ok(0) | Err(_)));
    }

    #[test]
    fn host_port_parsing() {
        assert_eq!(
            split_host_port("a.com:443", 80),
            Some(("a.com".into(), 443))
        );
        assert_eq!(split_host_port("a.com", 80), Some(("a.com".into(), 80)));
        assert_eq!(
            split_host_port("[::1]:8443", 80),
            Some(("::1".into(), 8443))
        );
        assert_eq!(split_host_port("", 80), None);
        assert_eq!(split_host_port("a.com:x", 80), None);
    }
}
