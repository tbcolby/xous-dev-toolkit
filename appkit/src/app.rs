//! App registration and lifecycle.
//!
//! There are two different names in every app, and mixing them up is the
//! single most common reason an app never appears in the menu:
//!
//! * **Server name** — registered with xous-names, e.g. `"_Mines_"`. Any unique string.
//! * **App name** — given to GAM in `UxRegistration.app_name`. It **must** be the
//!   generated constant `gam::APP_NAME_<KEY>`, where `<KEY>` is the app's key in
//!   `apps/manifest.json` (= the Cargo package name) uppercased with `-` → `_`.
//!   GAM refuses any name not in its generated list and `register_ux` returns `Ok(None)`.

use gam::{FocusState, UxRegistration, UxType};

use crate::screen::Screen;

/// Opcodes in the app's own main loop that GAM should call.
pub struct Opcodes {
    pub redraw: u32,
    /// Raw key presses. `None` for a `Chat` app that only wants IME lines.
    pub rawkeys: Option<u32>,
    /// Focus changes. Strongly recommended: stop pumps when backgrounded.
    pub focus_change: Option<u32>,
    /// Full input lines from the IME (Chat apps).
    pub got_input: Option<u32>,
}

/// Everything an app needs after registering.
pub struct Registered {
    pub screen: Screen,
    pub token: [u32; 4],
}

/// Register a UX with GAM and acquire its content canvas.
///
/// `app_name` must be `gam::APP_NAME_<KEY>` (see module docs). Use
/// `UxType::Framebuffer` for full-screen games and tools (gets the whole
/// canvas below the status bar and raw keys); `UxType::Chat` reserves an IME
/// input line at the bottom.
///
/// Panics with an explanatory message if GAM rejects the name.
pub fn register(
    xns: &xous_names::XousNames,
    sid: xous::SID,
    app_name: &str,
    ux_type: UxType,
    ops: Opcodes,
) -> Registered {
    let gam = gam::Gam::new(xns).expect("can't connect to GAM");
    let token = gam
        .register_ux(UxRegistration {
            app_name: String::from(app_name),
            ux_type,
            predictor: None,
            listener: sid.to_array(),
            redraw_id: ops.redraw,
            gotinput_id: ops.got_input,
            audioframe_id: None,
            rawkeys_id: ops.rawkeys,
            focuschange_id: ops.focus_change,
        })
        .expect("register_ux IPC failed")
        .unwrap_or_else(|| {
            panic!(
                "GAM rejected app name {:?}. It must be gam::APP_NAME_<KEY> generated from \
                 apps/manifest.json, not the xous-names server name.",
                app_name
            )
        });
    let gid = gam.request_content_canvas(token).expect("couldn't get content canvas");
    let screen = Screen::new(gam, gid);
    Registered { screen, token }
}

/// Decode the argument of a GAM focus-change message.
pub fn focus_state(code: usize) -> FocusState { FocusState::convert_focus_change(code) }

/// Leave the app: hand focus back to GAM (the previous context / main menu).
///
/// Xous apps are launched once at boot and are not respawned, so do **not**
/// `terminate_process` on a user "quit" — the app would vanish until reboot.
pub fn leave(screen: &Screen) {
    screen.gam().relinquish_focus().ok();
}

/// Tear down a server at the end of `main` (only on a real shutdown path).
pub fn shutdown(xns: &xous_names::XousNames, sid: xous::SID) -> ! {
    xns.unregister_server(sid).ok();
    // safety: called after the main loop exits; no blocking messages are pending
    // and no memory is lent out.
    unsafe {
        xous::destroy_server(sid).ok();
    }
    xous::terminate_process(0)
}
