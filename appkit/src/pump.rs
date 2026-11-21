//! A focus-aware periodic pump, modelled on `apps/ball` in xous-core but with
//! two of its edge cases closed:
//!
//! * `run()` while already running does not start a second tick chain
//!   (ball doubles its rate if it is foregrounded twice without a stop).
//! * A `stop()` followed quickly by `run()` cannot resurrect a stale tick:
//!   each run gets a generation number and old ticks are discarded.
//!
//! Each tick is delivered to the app's main server as a **blocking scalar**
//! with the opcode you choose, so ticks can never queue up faster than the app
//! draws. The app **must** answer it with [`ack`] (or `xous::return_scalar`),
//! otherwise the pump thread stays blocked.
//!
//! ```ignore
//! let pump = Pump::spawn(xous::connect(sid).unwrap(), AppOp::Tick as usize, 100);
//! // in the main loop:
//! Some(AppOp::Tick) => { app.step(); app.draw(); pump::ack(&msg); }
//! Some(AppOp::FocusChange) => match focus { Foreground => pump.run(), Background => pump.stop() },
//! Some(AppOp::Quit) => { pump.quit(); break; }
//! ```

use xous::{Message, CID, SID};

#[derive(Clone, Copy)]
#[repr(usize)]
enum Op {
    Run = 0,
    Stop = 1,
    Tick = 2,
    SetInterval = 3,
    Quit = 4,
}

impl Op {
    fn from(id: usize) -> Option<Op> {
        Some(match id {
            0 => Op::Run,
            1 => Op::Stop,
            2 => Op::Tick,
            3 => Op::SetInterval,
            4 => Op::Quit,
            _ => return None,
        })
    }
}

/// Handle to a pump thread. Dropping it does not stop the thread; call [`quit`](Pump::quit).
pub struct Pump {
    cid: CID,
}

impl Pump {
    /// Spawn a stopped pump that sends `opcode` to `main_cid` every `interval_ms`
    /// while running.
    pub fn spawn(main_cid: CID, opcode: usize, interval_ms: usize) -> Pump {
        let sid = xous::create_server().expect("pump: couldn't create server");
        let cid = xous::connect(sid).expect("pump: couldn't connect to own server");
        std::thread::spawn(move || pump_thread(sid, main_cid, opcode, interval_ms));
        Pump { cid }
    }

    /// Start ticking (call on `FocusState::Foreground`). Idempotent.
    pub fn run(&self) { self.send(Op::Run, 0); }

    /// Stop ticking (call on `FocusState::Background`). Idempotent.
    pub fn stop(&self) { self.send(Op::Stop, 0); }

    /// Change the tick interval; takes effect from the next tick.
    pub fn set_interval(&self, ms: usize) { self.send(Op::SetInterval, ms); }

    /// Stop the thread and wait for it to exit.
    pub fn quit(self) {
        xous::send_message(self.cid, Message::new_blocking_scalar(Op::Quit as usize, 0, 0, 0, 0))
            .expect("pump: couldn't send quit");
        // safety: the pump server has acknowledged Quit and holds no lent memory
        unsafe { xous::disconnect(self.cid).ok() };
    }

    fn send(&self, op: Op, arg: usize) {
        xous::send_message(self.cid, Message::new_scalar(op as usize, arg, 0, 0, 0))
            .expect("pump: couldn't send control message");
    }
}

/// Acknowledge a tick so the pump can schedule the next one.
pub fn ack(msg: &xous::MessageEnvelope) {
    if msg.body.is_blocking() {
        xous::return_scalar(msg.sender, 0).ok();
    }
}

fn pump_thread(sid: SID, main_cid: CID, opcode: usize, mut interval_ms: usize) {
    let tt = ticktimer_server::Ticktimer::new().expect("pump: no ticktimer");
    let self_cid = xous::connect(sid).expect("pump: couldn't connect to self");
    let mut running = false;
    let mut generation: usize = 0;
    loop {
        let msg = xous::receive_message(sid).expect("pump: receive failed");
        let args = msg.body.scalar_message().map(|s| (s.arg1, s.arg2)).unwrap_or((0, 0));
        match Op::from(msg.body.id()) {
            Some(Op::Run) => {
                if !running {
                    running = true;
                    generation = generation.wrapping_add(1);
                    let _ = xous::send_message(
                        self_cid,
                        Message::new_scalar(Op::Tick as usize, generation, 0, 0, 0),
                    );
                }
            }
            Some(Op::Stop) => running = false,
            Some(Op::SetInterval) => interval_ms = args.0.max(1),
            Some(Op::Tick) => {
                if running && args.0 == generation {
                    // Blocks until the app acks, so ticks never outrun drawing.
                    let _ = xous::send_message(
                        main_cid,
                        Message::new_blocking_scalar(opcode, 0, 0, 0, 0),
                    );
                    tt.sleep_ms(interval_ms).ok();
                    if running {
                        let _ = xous::send_message(
                            self_cid,
                            Message::new_scalar(Op::Tick as usize, generation, 0, 0, 0),
                        );
                    }
                }
            }
            Some(Op::Quit) => {
                xous::return_scalar(msg.sender, 0).ok();
                break;
            }
            None => log::warn!("pump: unknown opcode {}", msg.body.id()),
        }
    }
    // safety: the only connections are ours and the app's handle, which quit() disconnects
    unsafe {
        xous::disconnect(self_cid).ok();
        xous::destroy_server(sid).ok();
    }
}
