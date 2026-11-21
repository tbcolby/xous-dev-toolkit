//! Import lines and tiny blocks quoted in the docs that aren't otherwise exercised verbatim.
#![allow(unused_imports)]
use gam::menu::Circle;
use gam::menu::Line;
use gam::menu::RoundedRectangle;
use gam::menu::{DrawStyle, PixelColor};
use gam::menu::{Point, Rectangle};
use gam::menu::{TextBounds, TextView};
use gam::{Gam, GamObjectList, GamObjectType, GlyphStyle};
use std::io::Read;
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::AtomicU32;

// STANDARDS.md section 5
#[derive(Debug, Clone, PartialEq)]
enum AppState {
    // Every app starts here
    MainMenu,
    // App-specific states...
    Help,
    Settings,
}

// discoveries.md DrawStyle example
fn stroke() -> DrawStyle {
    DrawStyle {
        fill_color: None,
        stroke_color: Some(PixelColor::Dark),
        stroke_width: 3, // isize, not u16
    }
}
