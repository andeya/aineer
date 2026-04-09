use egui::IconData;

pub const APP_NAME: &str = "Aineer";
pub const APP_ID: &str = "aineer";
pub const APP_TAGLINE: &str = "AI-native terminal · agent as brain";

const ICON_SIZE: u32 = 256;

const BG: [u8; 4] = [13, 13, 31, 255];
const INDIGO_LIGHT: [u8; 3] = [129, 140, 248];
const INDIGO: [u8; 3] = [99, 102, 241];
const CYAN: [u8; 3] = [6, 182, 212];
const AMBER: [u8; 4] = [251, 191, 36, 230];
const BORDER: [u8; 4] = [99, 102, 241, 30];
const GLOW: [u8; 4] = [99, 102, 241, 50];

pub fn app_icon() -> IconData {
    let mut pixels = vec![0_u8; (ICON_SIZE * ICON_SIZE * 4) as usize];

    let bg_rect = Rect {
        left: 0,
        top: 0,
        width: ICON_SIZE,
        height: ICON_SIZE,
        radius: 54.0,
    };
    fill_rounded_rect(&mut pixels, bg_rect, BG);
    paint_glow(&mut pixels, 128.0, 120.0, 90.0, GLOW);
    stroke_rounded_rect(&mut pixels, bg_rect, 2.0, BORDER);

    #[rustfmt::skip]
    let outer_a: [(f32, f32); 7] = [
        (128.0, 38.0),                       // apex
        (218.0, 218.0), (190.0, 218.0),      // right foot
        (161.0, 160.0), (95.0, 160.0),       // crossbar gap
        (66.0, 218.0),  (38.0, 218.0),       // left foot
    ];
    let counter: [(f32, f32); 3] = [(128.0, 94.0), (104.0, 142.0), (152.0, 142.0)];

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let px = coord(x);
            let py = coord(y);
            let in_outer = point_in_polygon(px, py, &outer_a);
            let in_counter = point_in_polygon(px, py, &counter);
            if in_outer && !in_counter {
                let color = a_gradient(py);
                set_pixel(&mut pixels, x, y, color);
            }
        }
    }

    fill_circle(&mut pixels, 166.0, 148.0, 6.0, AMBER);

    IconData {
        rgba: pixels,
        width: ICON_SIZE,
        height: ICON_SIZE,
    }
}

fn a_gradient(y: f32) -> [u8; 4] {
    let t = ((y - 38.0) / 180.0).clamp(0.0, 1.0);
    let (r, g, b) = if t < 0.45 {
        let s = t / 0.45;
        lerp3(INDIGO_LIGHT, INDIGO, s)
    } else {
        let s = (t - 0.45) / 0.55;
        lerp3(INDIGO, CYAN, s)
    };
    [r, g, b, 255]
}

fn lerp3(a: [u8; 3], b: [u8; 3], t: f32) -> (u8, u8, u8) {
    let l = |i: usize| -> u8 {
        let v = f32::from(a[i]) * (1.0 - t) + f32::from(b[i]) * t;
        to_u8(v)
    };
    (l(0), l(1), l(2))
}

fn point_in_polygon(px: f32, py: f32, vertices: &[(f32, f32)]) -> bool {
    let mut crossings = 0;
    let n = vertices.len();
    for i in 0..n {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % n];
        if (y1 <= py && y2 > py) || (y2 <= py && y1 > py) {
            let t = (py - y1) / (y2 - y1);
            if px < x1 + t * (x2 - x1) {
                crossings += 1;
            }
        }
    }
    crossings % 2 == 1
}

#[allow(clippy::cast_precision_loss)]
fn coord(v: u32) -> f32 {
    v as f32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn set_pixel(buf: &mut [u8], x: u32, y: u32, c: [u8; 4]) {
    let i = ((y * ICON_SIZE + x) * 4) as usize;
    buf[i..i + 4].copy_from_slice(&c);
}

fn blend_pixel(buf: &mut [u8], x: u32, y: u32, c: [u8; 4], alpha: f32) {
    let i = ((y * ICON_SIZE + x) * 4) as usize;
    let a = alpha.clamp(0.0, 1.0) * (f32::from(c[3]) / 255.0);
    let keep = 1.0 - a;
    for ch in 0..3 {
        let mixed = f32::from(buf[i + ch]) * keep + f32::from(c[ch]) * a;
        buf[i + ch] = to_u8(mixed);
    }
    buf[i + 3] = 255;
}

fn fill_circle(buf: &mut [u8], cx: f32, cy: f32, r: f32, c: [u8; 4]) {
    let r2 = r * r;
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let dx = coord(x) - cx;
            let dy = coord(y) - cy;
            if dx * dx + dy * dy <= r2 {
                set_pixel(buf, x, y, c);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Rect {
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    radius: f32,
}

fn fill_rounded_rect(buf: &mut [u8], r: Rect, c: [u8; 4]) {
    let (l, t, ri, bo) = (
        coord(r.left),
        coord(r.top),
        coord(r.left + r.width),
        coord(r.top + r.height),
    );
    let rad = r.radius;
    let rad2 = rad * rad;
    for y in r.top..r.top + r.height {
        for x in r.left..r.left + r.width {
            let px = coord(x);
            let py = coord(y);
            let inside = corner_check(px, py, l, t, ri, bo, rad, rad2);
            if inside {
                set_pixel(buf, x, y, c);
            }
        }
    }
}

fn stroke_rounded_rect(buf: &mut [u8], r: Rect, thick: f32, c: [u8; 4]) {
    let (l, t, ri, bo) = (
        coord(r.left),
        coord(r.top),
        coord(r.left + r.width),
        coord(r.top + r.height),
    );
    let rad = r.radius;
    for y in r.top..r.top + r.height {
        for x in r.left..r.left + r.width {
            let px = coord(x);
            let py = coord(y);
            let outer = rr_dist(px, py, l, t, ri, bo, rad);
            let inner = rr_dist(
                px,
                py,
                l + thick,
                t + thick,
                ri - thick,
                bo - thick,
                (rad - thick).max(0.0),
            );
            if outer <= 0.0 && inner > 0.0 {
                set_pixel(buf, x, y, c);
            }
        }
    }
}

fn rr_dist(x: f32, y: f32, l: f32, t: f32, r: f32, b: f32, rad: f32) -> f32 {
    let qx = (x - x.clamp(l + rad, r - rad)).abs();
    let qy = (y - y.clamp(t + rad, b - rad)).abs();
    (qx * qx + qy * qy).sqrt() - rad
}

#[allow(clippy::too_many_arguments)]
fn corner_check(px: f32, py: f32, l: f32, t: f32, r: f32, b: f32, rad: f32, rad2: f32) -> bool {
    if px < l + rad && py < t + rad {
        let (dx, dy) = (px - l - rad, py - t - rad);
        dx * dx + dy * dy <= rad2
    } else if px > r - rad && py < t + rad {
        let (dx, dy) = (px - r + rad, py - t - rad);
        dx * dx + dy * dy <= rad2
    } else if px < l + rad && py > b - rad {
        let (dx, dy) = (px - l - rad, py - b + rad);
        dx * dx + dy * dy <= rad2
    } else if px > r - rad && py > b - rad {
        let (dx, dy) = (px - r + rad, py - b + rad);
        dx * dx + dy * dy <= rad2
    } else {
        true
    }
}

fn paint_glow(buf: &mut [u8], cx: f32, cy: f32, radius: f32, c: [u8; 4]) {
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let d = ((coord(x) - cx).powi(2) + (coord(y) - cy).powi(2)).sqrt();
            if d <= radius {
                let alpha = 1.0 - (d / radius);
                blend_pixel(buf, x, y, c, alpha * 0.5);
            }
        }
    }
}
