//! networking.md snippets
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use tls::xtls::TlsConnector;

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum NetworkOp {
    FetchComplete = 30,
    SyncComplete,
    DataReceived,
}

fn fetch_data(host: &str, port: u16) -> Result<Vec<u8>, std::io::Error> {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}

fn start_server(port: u16) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    listener.set_nonblocking(true)?;
    match listener.accept() {
        Ok((mut stream, addr)) => {
            log::info!("Connection from {}", addr);
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf)?;
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(e) => return Err(e),
    }
    Ok(())
}

fn udp_example() -> Result<(), std::io::Error> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(b"hello", "192.168.1.100:5000")?;
    let mut buf = [0u8; 1024];
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    let (len, src) = socket.recv_from(&mut buf)?;
    log::info!("Received {} bytes from {}", len, src);
    Ok(())
}

fn create_http_agent() -> ureq::Agent {
    ureq::builder().tls_connector(Arc::new(TlsConnector {})).timeout(Duration::from_secs(30)).build()
}

fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, ureq::Error> {
    let agent = create_http_agent();
    let response =
        agent.get(url).set("Accept", "application/json").set("User-Agent", "Precursor/1.0").call()?;
    let data: T = response.into_json()?;
    Ok(data)
}

fn post_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(url: &str, body: &T) -> Result<R, ureq::Error> {
    let agent = create_http_agent();
    let response = agent.post(url).set("Accept", "application/json").send_json(body)?; // sets Content-Type
    let data: R = response.into_json()?;
    Ok(data)
}

fn fetch_with_auth(url: &str, token: &str) -> Result<serde_json::Value, ureq::Error> {
    let agent = create_http_agent();
    let response = agent
        .get(url)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Accept", "application/json")
        .call()?;
    Ok(response.into_json()?)
}

fn error_handling(agent: &ureq::Agent, url: &str) -> Result<(), ureq::Error> {
    match agent.get(url).call() {
        Ok(response) => {
            let body: serde_json::Value = response.into_json()?;
        }
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            log::warn!("HTTP {}: {}", code, body);
            match code {
                401 => { /* Unauthorized - refresh token? */ }
                404 => { /* Not found */ }
                429 => { /* Rate limited - back off */ }
                500..=599 => { /* Server error - retry? */ }
                _ => {}
            }
        }
        Err(ureq::Error::Transport(e)) => {
            log::warn!("Network error: {:?}", e.kind());
            match e.kind() {
                ureq::ErrorKind::Dns => { /* DNS resolution failed */ }
                ureq::ErrorKind::ConnectionFailed => { /* Can't connect */ }
                ureq::ErrorKind::Io => { /* I/O error during transfer */ }
                _ => {}
            }
        }
    }
    Ok(())
}

fn fetch_with_retry<T: serde::de::DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
    max_retries: u32,
) -> Result<T, ureq::Error> {
    let mut attempt = 0;
    loop {
        match agent.get(url).call() {
            Ok(resp) => return Ok(resp.into_json()?),
            Err(e) if attempt + 1 >= max_retries => return Err(e),
            Err(e) => {
                log::warn!("Attempt {}/{} failed: {:?}", attempt + 1, max_retries, e);
                std::thread::sleep(Duration::from_millis(100 * 2_u64.pow(attempt))); // exponential backoff
                attempt += 1;
            }
        }
    }
}

fn spawn_fetch(cid: xous::CID, url: String) {
    std::thread::spawn(move || {
        let agent = create_http_agent();
        // Scalars carry 4 usizes only: put the body in shared state (Arc<Mutex<..>>) or
        // send it as a memory message (xous_ipc::Buffer), then signal completion.
        let (ok, len) = match agent.get(&url).call().map(|r| r.into_string()) {
            Ok(Ok(body)) => (1, body.len()),
            _ => (0, 0),
        };
        xous::send_message(cid, xous::Message::new_scalar(NetworkOp::FetchComplete as usize, ok, len, 0, 0)).ok();
    });
}

// ---- Polling Sync Thread ----
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Clone, serde::Deserialize)]
struct SyncData {
    value: u32,
}

struct SyncThread {
    running: Arc<AtomicBool>,
    data: Arc<Mutex<Option<SyncData>>>,
}

impl SyncThread {
    fn start(cid: xous::CID, interval_ms: usize) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let data = Arc::new(Mutex::new(None));
        let running_clone = running.clone();
        let data_clone = data.clone();

        std::thread::spawn(move || {
            let tt = ticktimer_server::Ticktimer::new().unwrap();
            let agent = create_http_agent();
            while running_clone.load(Ordering::Relaxed) {
                if let Ok(resp) = agent.get("https://api.example.com/sync").call() {
                    if let Ok(new_data) = resp.into_json::<SyncData>() {
                        *data_clone.lock().unwrap() = Some(new_data);
                        xous::send_message(
                            cid,
                            xous::Message::new_scalar(NetworkOp::SyncComplete as usize, 0, 0, 0, 0),
                        )
                        .ok();
                    }
                }
                tt.sleep_ms(interval_ms).ok(); // stop() takes effect after the current sleep
            }
        });
        Self { running, data }
    }

    fn stop(&self) { self.running.store(false, Ordering::Relaxed); }

    fn get_data(&self) -> Option<SyncData> { self.data.lock().unwrap().clone() }
}

fn spawn_tcp_receiver(cid: xous::CID, port: u16) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind: {:?}", e);
                return;
            }
        };
        log::info!("Listening on port {}", port);
        loop {
            match listener.accept() {
                Ok((mut stream, addr)) => {
                    log::info!("Connection from {}", addr);
                    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
                    let mut data = Vec::new();
                    if stream.read_to_end(&mut data).is_ok() {
                        xous::send_message(
                            cid,
                            xous::Message::new_scalar(NetworkOp::DataReceived as usize, data.len(), 0, 0, 0),
                        )
                        .ok();
                    }
                }
                Err(e) => log::warn!("Accept failed: {:?}", e),
            }
        }
    });
}

fn wifi_check() {
    let netmgr = net::NetManager::new();
    if let Some(ipv4) = netmgr.get_ipv4_config() {
        log::info!("Connected, IP: {:?}", ipv4.addr);
    } else {
        log::warn!("No network connection");
    }
}

// ---- Offline-first ----
struct PendingRequest;
impl PendingRequest {
    fn execute(&self, _a: &ureq::Agent) -> Result<(), ureq::Error> { Ok(()) }
}
struct SyncQueue {
    pending: Vec<PendingRequest>,
}

impl SyncQueue {
    fn save_to_pddb(&self) {}

    fn enqueue(&mut self, req: PendingRequest) {
        self.pending.push(req);
        self.save_to_pddb(); // survive restarts
    }

    fn flush(&mut self, agent: &ureq::Agent) {
        // Requests are sent in order; stop at the first failure and keep the rest.
        let sent = self.pending.iter().take_while(|req| req.execute(agent).is_ok()).count();
        self.pending.drain(..sent);
        self.save_to_pddb();
    }
}

#[derive(serde::Deserialize)]
struct Data;
fn cache_to_pddb(_p: &pddb::Pddb, _d: &Data) {}
fn load_from_pddb(_p: &pddb::Pddb) -> Option<Data> { None }

fn fetch_or_cache(pddb: &pddb::Pddb, agent: &ureq::Agent, url: &str) -> Option<Data> {
    match agent.get(url).call() {
        Ok(resp) => {
            let data: Data = resp.into_json().ok()?;
            cache_to_pddb(pddb, &data);
            Some(data)
        }
        Err(_) => load_from_pddb(pddb),
    }
}

struct Poller {
    is_charging: bool,
    is_foreground: bool,
}
impl Poller {
    fn sync_interval_ms(&self) -> usize {
        if self.is_charging {
            10_000
        } else if self.is_foreground {
            60_000
        } else {
            300_000
        }
    }
}
