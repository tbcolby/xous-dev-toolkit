//! Compile check for the xous-dev-toolkit knowledge docs.
//!
//! Every non-trivial fenced Rust snippet in `agents/*.md`, `discoveries.md` and
//! `ecosystem/STANDARDS.md` is reproduced here (with minimal scaffolding) so that
//! `cargo xtask hosted-ci docsnip` proves it compiles against the pinned xous-core.
//! The only systematic substitution: docs say `gam::APP_NAME_MYAPP`, this crate
//! uses `gam::APP_NAME_DOCSNIP` (the constant generated for this crate's manifest key).
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![allow(dead_code, unused_imports, unused_variables, unused_mut, unreachable_code)]

mod arch;
mod encoding;
mod gfx;
mod imports;
mod keys;
mod net_snips;
mod review;
mod rng;
mod storage;
mod system;

use num_traits::*;

// ---- architecture.md "App Lifecycle" (this IS the snippet, verbatim) ----

/// xous-names server name: any unique string. NOT the GAM app name.
const SERVER_NAME: &str = "_MyApp_";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
}

fn main() -> ! {
    // 1. Init logging
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);

    // 2. Register a server with the name server
    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    // 3. Connect to services (GAM, PDDB, etc.)
    let gam = gam::Gam::new(&xns).expect("can't connect to GAM");

    // 4. Register UX. app_name MUST be the constant generated from apps/manifest.json.
    let token = gam
        .register_ux(gam::UxRegistration {
            app_name: String::from(gam::APP_NAME_DOCSNIP),
            ux_type: gam::UxType::Framebuffer, // raw keys, no IME line
            predictor: None,
            listener: sid.to_array(),
            redraw_id: AppOp::Redraw.to_u32().unwrap(),
            gotinput_id: None,
            audioframe_id: None,
            rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
            focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
        })
        .expect("couldn't register UX")
        .expect("app name not in EXPECTED_APP_CONTEXTS");
    let gid = gam.request_content_canvas(token).expect("couldn't get content canvas");

    // 5. Main event loop. Apps are not respawned, so this loop never exits:
    //    "quit" means handing focus back to the main menu.
    let mut allow_redraw = true;
    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                if allow_redraw { /* draw, then gam.redraw() */ }
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                for k in [k1, k2, k3, k4] {
                    let key = char::from_u32(k as u32).unwrap_or('\u{0}');
                    match key {
                        '\u{0}' => {}
                        'q' => gam.relinquish_focus().unwrap(),
                        _ => { /* app.handle_key(key) */ }
                    }
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, code, _, _, _, {
                allow_redraw = matches!(
                    gam::FocusState::convert_focus_change(code),
                    gam::FocusState::Foreground
                );
            }),
            None => log::error!("unknown opcode: {:?}", msg),
        }
    }
}
