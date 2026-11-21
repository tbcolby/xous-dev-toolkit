//! architecture.md snippets
use gam::menu::*;
use gam::Gam;
use num_traits::*;
use xous::Message;

// ---- Standard Opcode Pattern ----
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum AppOp {
    // === GAM callbacks (0-9) ===
    Redraw = 0,
    Rawkeys = 1,
    FocusChange = 2,

    // === Internal messages (10-99) ===
    Pump = 10,          // Timer tick
    DataLoaded = 11,    // Async load complete
    NetworkResult = 12, // HTTP response
    SendData = 13,
    ProcessData = 14,
    ProcessRequest = 15,

    // === User actions (100+) ===
    NewItem = 100,
    DeleteItem = 101,
    SaveItem = 102,

    // === Control ===
    Quit = 255,
}

// ---- Pattern: Enum-Based State ----
#[derive(Debug, Clone)]
pub struct Item;
#[derive(Debug, Clone)]
pub struct Settings;
pub enum Event {
    Select(usize),
    Edit,
    Save,
    Quit,
}

#[derive(Debug, Clone)]
enum AppState {
    // Simple states
    Loading,
    Ready,

    // States with data
    Viewing { item_id: usize },
    Editing { buffer: String, dirty: bool },

    // Nested states
    Menu { parent: Box<AppState>, selection: usize },
}

struct App {
    state: AppState,
    // Shared resources
    items: Vec<Item>,
    settings: Settings,
}

impl App {
    fn transition(&mut self, event: Event) {
        self.state = match (&self.state, event) {
            (AppState::Ready, Event::Select(id)) => AppState::Viewing { item_id: id },
            (AppState::Viewing { .. }, Event::Edit) => {
                AppState::Editing { buffer: String::new(), dirty: false }
            }
            (AppState::Editing { .. }, Event::Save) => AppState::Ready,
            (_, Event::Quit) => AppState::Ready,
            _ => return, // No transition
        };
    }
}

// ---- Pattern: Screen-Based State ----
enum Screen {
    ModeSelect,
    ItemList,
    ItemDetail(usize),
    Editor { doc_id: usize },
    Settings,
}

struct ScreenApp {
    screen: Screen,
}

impl ScreenApp {
    fn render(&self, gam: &Gam, gid: Gid) {
        match &self.screen {
            Screen::ModeSelect => self.render_mode_select(gam, gid),
            Screen::ItemList => self.render_item_list(gam, gid),
            _ => {} // ... one arm per screen
        }
    }

    fn handle_key(&mut self, key: char) -> Option<Screen> {
        match &self.screen {
            Screen::ModeSelect => self.handle_mode_select_key(key),
            Screen::ItemList => self.handle_item_list_key(key),
            _ => None, // ... one arm per screen
        }
    }
    // scaffolding
    fn render_mode_select(&self, _g: &Gam, _gid: Gid) {}
    fn render_item_list(&self, _g: &Gam, _gid: Gid) {}
    fn handle_mode_select_key(&mut self, _k: char) -> Option<Screen> { None }
    fn handle_item_list_key(&mut self, _k: char) -> Option<Screen> { None }
}

// ---- Message Unpacking Patterns ----
fn dispatch(msg: &xous::MessageEnvelope) {
    match FromPrimitive::from_usize(msg.body.id()) {
        // Scalar message (up to 4 usize args)
        Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
            // k1-k4 are up to four chars (as u32), 0 = no key
        }),

        // Blocking scalar (the sender is blocked until you reply)
        Some(AppOp::DataLoaded) => xous::msg_blocking_scalar_unpack!(msg, status, count, _, _, {
            xous::return_scalar(msg.sender, 0).ok();
        }),
        _ => log::warn!("unhandled opcode {:?}", msg.body.id()),
    }
}

// ---- Focus-aware pump (canonical, from apps/ball) ----
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum PumpOp {
    Run,
    Stop,
    Pump,
    Quit,
}

const PUMP_INTERVAL_MS: usize = 50; // 20 FPS

/// Spawns a thread that sends a *blocking* `AppOp::Pump` to the main loop every
/// PUMP_INTERVAL_MS while running. It re-arms itself by sending `PumpOp::Pump`
/// to its own server, so it never busy-waits and costs nothing when stopped.
pub fn spawn_pump(cid_to_main: xous::CID, pump_sid: xous::SID) {
    std::thread::spawn(move || {
        let tt = ticktimer_server::Ticktimer::new().unwrap();
        let cid_to_self = xous::connect(pump_sid).unwrap();
        let mut run = false;
        let mut generation: usize = 0; // bumped on every Run so a stale tick chain dies
        loop {
            let msg = xous::receive_message(pump_sid).unwrap(); // blocks; no CPU while idle
            match FromPrimitive::from_usize(msg.body.id()) {
                Some(PumpOp::Run) => {
                    if !run {
                        run = true;
                        generation = generation.wrapping_add(1);
                        xous::send_message(
                            cid_to_self,
                            Message::new_scalar(PumpOp::Pump as usize, generation, 0, 0, 0),
                        )
                        .unwrap();
                    }
                }
                Some(PumpOp::Stop) => run = false,
                Some(PumpOp::Pump) => xous::msg_scalar_unpack!(msg, tick_generation, _, _, _, {
                    // Ignore ticks from a chain started before the last Stop/Run, or
                    // a quick Stop+Run would double the frame rate (apps/ball has this bug).
                    if run && tick_generation == generation {
                        // blocking: waits until main loop has drawn the frame
                        xous::send_message(
                            cid_to_main,
                            Message::new_blocking_scalar(AppOp::Pump as usize, 0, 0, 0, 0),
                        )
                        .unwrap();
                        tt.sleep_ms(PUMP_INTERVAL_MS).unwrap();
                        xous::send_message(
                            cid_to_self,
                            Message::new_scalar(PumpOp::Pump as usize, generation, 0, 0, 0),
                        )
                        .unwrap();
                    }
                }),
                Some(PumpOp::Quit) => {
                    xous::return_scalar(msg.sender, 1).unwrap();
                    break;
                }
                None => log::error!("pump: unknown opcode {:?}", msg),
            }
        }
        // safety: server has no pending blocking messages or lent pages
        unsafe { xous::destroy_server(pump_sid).ok() };
    });
}

fn pump_setup_and_loop(sid: xous::SID) {
    // in main(), after registering `sid`:
    let pump_sid = xous::create_server().unwrap();
    let cid_to_pump = xous::connect(pump_sid).unwrap();
    spawn_pump(xous::connect(sid).unwrap(), pump_sid);

    let mut allow_redraw = true;
    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Pump) => {
                if allow_redraw { /* app.step(); app.draw(); gam.redraw() */ }
                xous::return_scalar(msg.sender, 1).unwrap(); // ack, or the pump stalls
            }
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, code, _, _, _, {
                let op = match gam::FocusState::convert_focus_change(code) {
                    gam::FocusState::Background => {
                        allow_redraw = false; // drops Pumps already queued
                        PumpOp::Stop
                    }
                    gam::FocusState::Foreground => {
                        allow_redraw = true;
                        PumpOp::Run
                    }
                };
                xous::send_message(cid_to_pump, Message::new_scalar(op as usize, 0, 0, 0, 0)).unwrap();
            }),
            _ => {}
        }
    }
}

// ---- Controllable pump helpers (from Timers) ----
struct Timers {
    pump_running: bool,
    pump_conn: xous::CID,
}
impl Timers {
    fn start_pump(&mut self) {
        if !self.pump_running {
            self.pump_running = true;
            xous::send_message(self.pump_conn, xous::Message::new_scalar(PumpOp::Run as usize, 0, 0, 0, 0)).ok();
        }
    }

    fn stop_pump(&mut self) {
        if self.pump_running {
            self.pump_running = false;
            xous::send_message(self.pump_conn, xous::Message::new_scalar(PumpOp::Stop as usize, 0, 0, 0, 0))
                .ok();
        }
    }
}

// ---- Controllable Background Thread (mpsc) ----
pub enum ThreadCmd {
    Start,
    Stop,
    Quit,
}

fn spawn_worker(cid: xous::CID) -> std::sync::mpsc::Sender<ThreadCmd> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let tt = ticktimer_server::Ticktimer::new().unwrap();
        let mut running = false;
        loop {
            // Stopped: block (no CPU). Running: poll between work items.
            let cmd = if running { rx.try_recv().ok() } else { rx.recv().ok() };
            match cmd {
                Some(ThreadCmd::Start) => running = true,
                Some(ThreadCmd::Stop) => running = false,
                Some(ThreadCmd::Quit) => break,
                None => {}
            }
            if running {
                // Do work, then notify the main loop
                xous::send_message(cid, Message::new_scalar(AppOp::DataLoaded as usize, 0, 0, 0, 0)).ok();
                tt.sleep_ms(100).ok();
            }
        }
    });

    tx // keep this; send ThreadCmd::Stop on FocusState::Background
}

// ---- Network Thread Pattern ----
fn fetch_data() -> Result<Vec<u8>, std::io::Error> { Ok(Vec::new()) }

fn spawn_network_worker(cid: xous::CID) {
    std::thread::spawn(move || {
        // Network operations here (blocking OK)
        let (ok, len) = match fetch_data() {
            Ok(data) => (1, data.len()),
            Err(_) => (0, 0),
        };
        xous::send_message(cid, Message::new_scalar(AppOp::NetworkResult as usize, ok, len, 0, 0)).ok();
    });
}

// ---- Memory Message Patterns ----
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct MyStruct {
    pub value: u32,
}

fn send_memory(cid: xous::CID) {
    use xous_ipc::Buffer;

    // Lend (read-only, caller retains ownership)
    let my_string = String::from("hello");
    let buf = Buffer::into_buf(my_string).unwrap();
    buf.lend(cid, AppOp::SendData.to_u32().unwrap()).unwrap();

    // Lend mutable (callee can modify, caller gets it back)
    let my_struct = MyStruct { value: 1 };
    let mut buf = Buffer::into_buf(my_struct).unwrap();
    buf.lend_mut(cid, AppOp::ProcessData.to_u32().unwrap()).unwrap();
    let result: MyStruct = buf.to_original::<MyStruct, _>().unwrap();
}

fn receive_memory(msg: &xous::MessageEnvelope) {
    use xous_ipc::Buffer;
    match FromPrimitive::from_usize(msg.body.id()) {
        Some(AppOp::SendData) => {
            let buffer = unsafe { Buffer::from_memory_message(msg.body.memory_message().unwrap()) };
            let data = buffer.to_original::<String, _>().unwrap();
            log::info!("Received: {}", data);
        }
        _ => {}
    }
}

// ---- Shared State with Arc/Mutex ----
#[derive(Debug, Clone)]
enum AppMode {
    Default,
}
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum WorkerOp {
    DoWork,
}

fn shared_state(worker_sid: xous::SID) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    // Shared state between main and worker threads
    let mode = Arc::new(Mutex::new(AppMode::Default));
    let data_cache: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let is_busy = Arc::new(AtomicBool::new(false));

    // Spawn worker thread with cloned references
    let worker_mode = mode.clone();
    let worker_cache = data_cache.clone();
    let worker_busy = is_busy.clone();
    std::thread::spawn(move || loop {
        let msg = xous::receive_message(worker_sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(WorkerOp::DoWork) => {
                worker_busy.store(true, Ordering::SeqCst);
                let current_mode = worker_mode.lock().unwrap().clone();
                // ... do work based on mode ...
                worker_busy.store(false, Ordering::SeqCst);
            }
            _ => {}
        }
    });
}

// ---- RV32 Atomic Limitation ----
fn split_u64(val: u64) -> u64 {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let value_msb = Arc::new(AtomicU32::new(0));
    let value_lsb = Arc::new(AtomicU32::new(0));

    // Write (not atomic as a pair: guard with a Mutex if readers must never see a torn value)
    value_msb.store((val >> 32) as u32, Ordering::SeqCst);
    value_lsb.store(val as u32, Ordering::SeqCst);

    // Read
    ((value_msb.load(Ordering::SeqCst) as u64) << 32) | (value_lsb.load(Ordering::SeqCst) as u64)
}

// ---- Custom Stack Size (as in apps/mtxcli, apps/chat-test) ----
fn stack_main() -> ! {
    // Wrap main in a thread with larger stack (default may be too small)
    std::thread::Builder::new()
        .stack_size(1024 * 1024) // 1 MB stack
        .spawn(wrapped_main)
        .unwrap()
        .join()
        .unwrap()
}

fn wrapped_main() -> ! {
    // Actual app logic here
    loop {
        xous::yield_slice();
    }
}

// ---- Heap Limit Adjustment (as in apps/mtxchat) ----
fn raise_heap() {
    const HEAP_LARGER_LIMIT: usize = 2048 * 1024; // 2 MB

    // Two-step syscall: first probe, then set
    let result =
        xous::rsyscall(xous::SysCall::AdjustProcessLimit(xous::Limits::HeapMaximum as usize, 0, HEAP_LARGER_LIMIT));
    if let Ok(xous::Result::Scalar2(1, current_limit)) = result {
        xous::rsyscall(xous::SysCall::AdjustProcessLimit(
            xous::Limits::HeapMaximum as usize,
            current_limit,
            HEAP_LARGER_LIMIT,
        ))
        .unwrap();
        log::info!("Heap limit increased to: {}", HEAP_LARGER_LIMIT);
    } else {
        log::warn!("Heap limit syscall unsupported: {:?}", result);
    }
}

// ---- Command-prefix pattern (replaces the unreachable ESC prefix) ----
const KEY_F1: char = '\u{11}';

#[derive(Clone, Copy)]
enum EditorMode {
    Editor,
    Menu,
}
struct Editor {
    cmd_pending: bool,
    mode: EditorMode,
}

impl Editor {
    fn handle_key(&mut self, key: char) {
        // Second key of an F1 command
        if self.cmd_pending {
            self.cmd_pending = false;
            self.handle_command(key);
            return;
        }

        // F1 starts a command sequence. Do NOT use ESC (no key produces 0x1B)
        // or '∴' (the GAM swallows it to open the main menu).
        if key == KEY_F1 {
            self.cmd_pending = true;
            return;
        }

        // Normal key handling per mode
        match self.mode {
            EditorMode::Editor => self.handle_editor_key(key),
            EditorMode::Menu => self.handle_menu_key(key),
        }
    }

    fn handle_command(&mut self, key: char) {
        match key {
            'p' => self.toggle_preview(), // F1, p = preview
            's' => self.save(),           // F1, s = save
            'e' => self.show_export(),    // F1, e = export menu
            'q' => self.quit_to_parent(), // F1, q = back
            _ => {}                       // Unknown command, ignore
        }
    }
    // scaffolding
    fn handle_editor_key(&mut self, _k: char) {}
    fn handle_menu_key(&mut self, _k: char) {}
    fn toggle_preview(&mut self) {}
    fn save(&mut self) {}
    fn show_export(&mut self) {}
    fn quit_to_parent(&mut self) {}
}

// ---- Error Propagation in IPC ----
fn process(arg: usize) -> Result<usize, ()> { Ok(arg) }

fn callee(msg: &xous::MessageEnvelope) {
    match FromPrimitive::from_usize(msg.body.id()) {
        Some(AppOp::ProcessRequest) => xous::msg_blocking_scalar_unpack!(msg, arg, _, _, _, {
            match process(arg) {
                Ok(value) => xous::return_scalar(msg.sender, value).ok(),
                Err(_) => xous::return_scalar(msg.sender, usize::MAX).ok(), // Error sentinel
            };
        }),
        _ => {}
    }
}

fn caller(cid: xous::CID, arg: usize) -> Result<(), xous::Error> {
    let response =
        xous::send_message(cid, Message::new_blocking_scalar(AppOp::ProcessRequest as usize, arg, 0, 0, 0))?;
    if let xous::Result::Scalar1(value) = response {
        if value == usize::MAX {
            log::error!("Request failed");
        }
    }
    Ok(())
}
