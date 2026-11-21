//! review.md / discoveries.md / STANDARDS.md registration & focus snippets
use num_traits::*;

const SERVER_NAME: &str = "_MyApp_"; // xous-names server name: any unique string

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys = 1,
    FocusChange = 2,
    Pump = 10,
    Quit = 255,
}

fn register(xns: &xous_names::XousNames, gam: &gam::Gam) -> Result<[u32; 4], xous::Error> {
    let sid = xns.register_name(SERVER_NAME, None)?;
    let token = gam
        .register_ux(gam::UxRegistration {
            // Generated from apps/manifest.json: key "myapp" -> APP_NAME_MYAPP, value = its context_name
            app_name: String::from(gam::APP_NAME_DOCSNIP),
            ux_type: gam::UxType::Framebuffer,
            predictor: None,
            listener: sid.to_array(),
            redraw_id: AppOp::Redraw.to_u32().unwrap(),
            gotinput_id: None,
            audioframe_id: None,
            rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
            focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
        })?
        .expect("GAM rejected app_name (not in EXPECTED_APP_CONTEXTS)");
    Ok(token)
}

fn focus(msg: &xous::MessageEnvelope, allow_redraw: &mut bool) {
    match FromPrimitive::from_usize(msg.body.id()) {
        Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, state_code, _, _, _, {
            match gam::FocusState::convert_focus_change(state_code) {
                gam::FocusState::Background => {
                    *allow_redraw = false;
                    // Stop pump threads, save state
                }
                gam::FocusState::Foreground => {
                    *allow_redraw = true;
                    // Resume pump threads, trigger redraw
                }
            }
        }),
        _ => {}
    }
}

// discoveries.md borrow-checker pattern
#[derive(Clone)]
struct Game;
#[derive(Clone, Copy)]
enum Mode {
    A,
}
enum State {
    Playing { game: Game, mode: Mode },
    GameOver { game: Game, mode: Mode },
}
struct App {
    state: State,
}
impl App {
    fn update_stats(&mut self, _m: Mode) {}
    fn finish(&mut self) {
        let data = if let State::Playing { game, mode } = &self.state { Some((game.clone(), *mode)) } else { None };
        if let Some((game, mode)) = data {
            self.update_stats(mode);
            self.state = State::GameOver { game, mode };
        }
    }
}
