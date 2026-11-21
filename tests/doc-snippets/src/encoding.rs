//! encoding.md snippets
use gam::menu::*;
use gam::Gam;
use gam::{GamObjectList, GamObjectType};

/// Largest module size (px) that fits, capped at 8. None if the code can't fit at >= 1 px.
fn auto_fit_size(modules: usize, display_w: usize, display_h: usize, quiet: usize) -> Option<usize> {
    let total_modules = modules + quiet * 2;
    let px = (display_w.min(display_h) / total_modules).min(8);
    if px == 0 { None } else { Some(px) }
}

fn push_rect(gam: &Gam, gid: Gid, list: &mut GamObjectList, r: Rectangle) -> Result<(), xous::Error> {
    if let Err(obj) = list.push(GamObjectType::Rect(r)) {
        // list full: flush and start a new batch
        let full = std::mem::replace(list, GamObjectList::new(gid));
        gam.draw_list(full)?;
        list.push(obj).ok();
    }
    Ok(())
}

/// `modules[row][col] == true` means a dark module. Square matrix.
fn draw_matrix(gam: &Gam, gid: Gid, canvas: Point, modules: &[Vec<bool>], quiet: usize) -> Result<(), xous::Error> {
    let size = modules.len();
    let (w, h) = (canvas.x as usize, canvas.y as usize);
    let Some(px) = auto_fit_size(size, w, h, quiet) else {
        log::warn!("code too large for the display");
        return Ok(());
    };
    let total_px = (size + quiet * 2) * px; // <= min(w, h), so the subtractions can't underflow
    let x0 = ((w - total_px) / 2 + quiet * px) as isize;
    let y0 = ((h - total_px) / 2 + quiet * px) as isize;
    let px = px as isize;
    let dark = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0);

    let mut list = GamObjectList::new(gid);
    for (row, line) in modules.iter().enumerate() {
        for (col, &is_dark) in line.iter().enumerate() {
            if is_dark {
                let (x, y) = (x0 + col as isize * px, y0 + row as isize * px);
                push_rect(gam, gid, &mut list, Rectangle::new_coords_with_style(x, y, x + px - 1, y + px - 1, dark))?;
            }
        }
    }
    gam.draw_list(list)?;
    gam.redraw()
}

fn draw_barcode(gam: &Gam, gid: Gid, bars: &[bool], x_start: isize, y_start: isize, bar_width: isize, height: isize)
    -> Result<(), xous::Error> {
    let dark = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0);
    let mut list = GamObjectList::new(gid);
    for (i, &is_dark) in bars.iter().enumerate() {
        if is_dark {
            let x = x_start + i as isize * bar_width;
            push_rect(gam, gid, &mut list, Rectangle::new_coords_with_style(x, y_start, x + bar_width - 1, y_start + height, dark))?;
        }
    }
    gam.draw_list(list)
}
