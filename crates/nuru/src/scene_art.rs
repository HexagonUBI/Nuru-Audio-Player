use std::fmt::Write;

const W: f64 = 1600.0;
const H: f64 = 900.0;
const DESK: f64 = 742.0;

struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0 as f64 / 4294967296.0
    }
}

pub fn sky(name: &str) -> [&'static str; 3] {
    match name {
        "night" => ["#080a14", "#131a2e", "#1b2136"],
        "morning" => ["#1a2436", "#4a5a6b", "#6b6553"],
        _ => ["#0d1016", "#171c26", "#1e232c"],
    }
}

pub fn view_colors(name: &str) -> [&'static str; 3] {
    match name {
        "forest" => ["#16281c", "#0f1f16", "#07110c"],
        "city" => ["#141c26", "#0d141d", "#070b10"],
        "beach" => ["#13303b", "#0d2531", "#071820"],
        _ => ["#151a20", "#0f1319", "#080a0d"],
    }
}

fn pine(x: f64, y: f64, w: f64, h: f64) -> String {
    let t = |f: f64| y - h * f;
    let o = |f: f64| x + w * f;
    format!(
        "M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z",
        x,
        y - h,
        o(0.26),
        t(0.58),
        o(0.14),
        t(0.58),
        o(0.4),
        t(0.26),
        o(0.22),
        t(0.26),
        o(0.5),
        y,
        o(-0.5),
        y,
        o(-0.22),
        t(0.26),
        o(-0.4),
        t(0.26),
        o(-0.14),
        t(0.58),
        o(-0.26),
        t(0.58)
    )
}

fn broadleaf(x: f64, y: f64, w: f64, h: f64) -> String {
    let r = w / 2.0;
    let cy = y - h + r * 0.85;
    format!(
        "M{:.1} {:.1}L{:.1} {:.1}a{:.1} {:.1} 0 1 1 {:.1} 0L{:.1} {:.1}Z",
        x - w * 0.045,
        y,
        x - w * 0.03,
        cy + r * 0.5,
        r,
        r * 0.86,
        w * 0.06,
        x + w * 0.045,
        y
    )
}

fn path(out: &mut String, d: &str, fill: &str, extra: &str) {
    if !d.is_empty() {
        let _ = write!(out, "<path d=\"{d}\" fill=\"{fill}\" {extra}/>");
    }
}

fn forest(out: &mut String, c: [&str; 3]) {
    let bands = [
        (640.0, 46, (70.0, 130.0), (46.0, 78.0), 0u32),
        (730.0, 32, (110.0, 200.0), (66.0, 108.0), 1),
        (900.0, 20, (190.0, 330.0), (104.0, 168.0), 2),
    ];
    for (by, count, hr, wr, depth) in bands {
        let mut r = Rng::new(1337 + depth * 977);
        let mut d = String::new();
        for i in 0..count {
            let x = -80.0 + ((W + 160.0) / (count - 1) as f64) * i as f64 + (r.next() - 0.5) * 44.0;
            let h = hr.0 + r.next() * (hr.1 - hr.0);
            let w = wr.0 + r.next() * (wr.1 - wr.0);
            d.push_str(&pine(x, by + r.next() * 12.0, w, h));
        }
        let extra = if depth == 0 { "filter=\"url(#soft)\"" } else { "" };
        path(out, &d, c[depth as usize], extra);
    }
}

fn city(out: &mut String, c: [&str; 3], wet: f64) {
    let bands = [
        (660.0, 26, (90.0, 210.0), (40.0, 92.0), 0u32),
        (760.0, 18, (150.0, 340.0), (64.0, 132.0), 1),
        (900.0, 11, (230.0, 430.0), (110.0, 210.0), 2),
    ];
    let lit = 0.55 - wet * 0.2;
    for (by, count, hr, wr, depth) in bands {
        let mut r = Rng::new(4242 + depth * 613);
        let mut x = -70.0;
        let mut blocks = String::new();
        let mut wins = String::new();
        for _ in 0..count {
            let w = wr.0 + r.next() * (wr.1 - wr.0);
            let h = hr.0 + r.next() * (hr.1 - hr.0);
            let top = by - h;
            let cols = ((w / 22.0).floor() as i32).max(2);
            let rows = ((h / 30.0).floor() as i32).max(3);
            if depth > 0 {
                for col in 0..cols {
                    for q in 0..rows {
                        if r.next() < 0.42 {
                            continue;
                        }
                        let wx = x + 8.0 + (col as f64 * (w - 16.0)) / cols as f64;
                        let wy = top + 14.0 + (q as f64 * (h - 26.0)) / rows as f64;
                        let _ = write!(wins, "M{wx:.1} {wy:.1}h7v10h-7Z");
                    }
                }
            }
            let _ = write!(blocks, "M{:.1} {:.1}h{:.1}v{:.1}h{:.1}Z", x, top, w, H - top, -w);
            x += w + 4.0 + r.next() * 16.0;
        }
        let extra = if depth == 0 { "filter=\"url(#soft)\"" } else { "" };
        path(out, &blocks, c[depth as usize], extra);
        if depth > 0 {
            path(out, &wins, "#ffd79a", &format!("opacity=\"{lit:.3}\""));
        }
    }
}

fn beach(out: &mut String, c: [&str; 3]) {
    let mut r = Rng::new(90210);
    let mut waves = String::new();
    for i in 0..16 {
        let y = 690.0 + i as f64 * 13.0;
        let amp = 5.0 + i as f64 * 0.9;
        let seg = 150.0 + r.next() * 90.0;
        let _ = write!(waves, "M-40 {y:.1}");
        let mut x = -40.0;
        while x < W + 80.0 {
            let _ = write!(waves, "q{:.1} {:.1} {:.1} 0", seg / 2.0, -amp, seg);
            x += seg;
        }
    }
    let mut palms = String::new();
    for (px, ph, dir) in [(190.0f64, 300.0f64, 1.0f64), (1430.0, 250.0, -1.0)] {
        let base = 900.0;
        let top_y = base - ph;
        let _ = write!(
            palms,
            "M{} {}q{} {} {} {}l10 2q{} {} {} {}Z",
            px - 9.0,
            base,
            6.0 * dir,
            -ph * 0.55,
            20.0 * dir,
            -ph,
            -12.0 * dir,
            ph * 0.56,
            -16.0 * dir,
            ph
        );
        for f in 0..6 {
            let a = -0.35 + f as f64 * 0.42;
            let lx = px + 20.0 * dir;
            let ex = lx + a.cos() * 118.0 * dir;
            let ey = top_y + a.sin() * 96.0 - 10.0;
            let _ = write!(
                palms,
                "M{} {}q{} {} {} {}q{} {} {} {}Z",
                lx,
                top_y,
                (ex - lx) * 0.55,
                (ey - top_y) * 0.2,
                ex - lx,
                ey - top_y,
                (lx - ex) * 0.4,
                (top_y - ey) * 0.55,
                lx - ex,
                top_y - ey
            );
        }
    }
    path(out, &format!("M-60 690H{}V{}H-60Z", W + 120.0, H), c[0], "filter=\"url(#soft)\"");
    let _ = write!(out, "<path d=\"{waves}\" fill=\"none\" stroke=\"rgb(190,225,255)\" stroke-opacity=\"0.16\" stroke-width=\"3\"/>");
    path(out, &format!("M-60 862q400 -26 820 -4t840 -10V{}H-60Z", H), c[2], "");
    path(out, &palms, c[2], "");
}

fn hills(out: &mut String, c: [&str; 3]) {
    let layer = |y: f64, amp: f64, seg: f64, seed: u32| {
        let mut r = Rng::new(seed);
        let mut d = format!("M-60 {}L-60 {:.1}", H, y);
        let mut x = -60.0;
        while x < W + 120.0 {
            let lift = -amp - r.next() * amp;
            let drift = (r.next() - 0.5) * amp * 0.5;
            let _ = write!(d, "q{:.1} {:.1} {:.1} {:.1}", seg / 2.0, lift, seg, drift);
            x += seg;
        }
        let _ = write!(d, "L{} {}Z", W + 120.0, H);
        d
    };
    let mut posts = String::new();
    for i in 0..14 {
        let x = 60.0 + i as f64 * 118.0;
        let y = 858.0 - (i as f64 * 0.7).sin() * 10.0;
        let _ = write!(posts, "M{x} {y}h9v56h-9Z");
    }
    posts.push_str("M60 812q560 26 1180 -6v12q-620 32 -1180 6Z");
    path(out, &layer(690.0, 40.0, 260.0, 7001), c[0], "filter=\"url(#soft)\"");
    path(out, &layer(760.0, 54.0, 220.0, 7333), c[1], "");
    path(out, &layer(846.0, 30.0, 300.0, 7777), c[2], "");
    path(out, &posts, c[2], "");
    path(out, &broadleaf(1240.0, 848.0, 210.0, 250.0), c[2], "");
}

pub fn view(out: &mut String, name: &str, wet: f64) {
    let c = view_colors(name);
    match name {
        "forest" => forest(out, c),
        "city" => city(out, c, wet),
        "beach" => beach(out, c),
        "hills" => hills(out, c),
        _ => {}
    }
}

fn shelf() -> (String, String) {
    let x0 = 26.0;
    let x1 = 250.0;
    let boards = [188.0, 330.0, 470.0, 604.0];
    let mut frame = format!(
        "M{} 0h{}v{}h-14V14h-{}v{}h-14Z",
        x0,
        x1 - x0,
        DESK,
        x1 - x0 - 28.0,
        DESK - 14.0
    );
    for y in boards {
        let _ = write!(frame, "M{} {}h{}v13h-{}Z", x0, y, x1 - x0, x1 - x0);
    }

    let mut r = Rng::new(20260818);
    let mut items = String::new();
    for (bi, y) in boards.iter().copied().enumerate() {
        let mut x = x0 + 20.0;
        let limit = x1 - 22.0;
        let mut guard = 0;
        while x < limit - 14.0 {
            let g = guard;
            guard += 1;
            if g >= 30 {
                break;
            }
            let kind = r.next();
            if kind < 0.68 {
                let w = 11.0 + r.next() * 15.0;
                let h = 52.0 + r.next() * 46.0;
                let lean = bi == 2 && x > x0 + 120.0 && r.next() < 0.4;
                if lean {
                    let _ = write!(
                        items,
                        "M{:.1} {}l{:.1} -{:.1}l{:.1} {:.1}l-{:.1} {:.1}Z",
                        x,
                        y,
                        w * 1.7,
                        h * 0.5,
                        w,
                        h * 0.24,
                        w * 1.7,
                        h * 0.5
                    );
                    x += w * 2.2;
                } else {
                    let _ = write!(items, "M{:.1} {:.1}h{:.1}v{:.1}h-{:.1}Z", x, y - h, w, h, w);
                    x += w + 3.0;
                }
            } else if kind < 0.86 {
                let w = 42.0 + r.next() * 30.0;
                let h = 34.0 + r.next() * 18.0;
                let _ = write!(items, "M{:.1} {:.1}h{:.1}v{:.1}h-{:.1}Z", x, y - h, w, h, w);
                let _ = write!(
                    items,
                    "M{:.1} {:.1}h{:.1}v4h-{:.1}Z",
                    x + 6.0,
                    y - h + 7.0,
                    w - 12.0,
                    w - 12.0
                );
                x += w + 8.0;
            } else {
                let rr = 15.0 + r.next() * 9.0;
                let _ = write!(
                    items,
                    "M{:.1} {}a{:.1} {:.1} 0 1 1 0 -0.1ZM{:.1} {:.1}h{:.1}v{:.1}h-{:.1}Z",
                    x + rr,
                    y,
                    rr,
                    rr,
                    x + rr * 0.55,
                    y - rr * 1.9,
                    rr * 0.9,
                    rr * 0.9,
                    rr * 0.9
                );
                x += rr * 2.0 + 10.0;
            }
        }
    }
    (frame, items)
}

fn hanger() -> (String, String, String) {
    let x = 366.0;
    let pot_top = 178.0;
    let pot = format!("M{} {}h92l-16 62h-60Z", x - 46.0, pot_top);
    let cords = format!("M{} {}L{} 0M{} {}L{} 0", x - 40.0, pot_top, x - 12.0, x + 40.0, pot_top, x + 12.0);
    let mut vines = String::new();
    let mut r = Rng::new(4471);
    for i in 0..7 {
        let sx = x - 38.0 + i as f64 * 12.6;
        let len = 90.0 + r.next() * 210.0;
        let sway = (r.next() - 0.5) * 70.0;
        let _ = write!(
            vines,
            "M{:.1} {}q{:.1} {:.1} {:.1} {:.1}",
            sx,
            pot_top + 30.0,
            sway,
            len * 0.55,
            sway * 0.4,
            len
        );
        for l in 1..5 {
            let t = l as f64 / 5.0;
            let lx = sx + sway * t * 0.8;
            let ly = pot_top + 30.0 + len * t;
            let _ = write!(vines, "M{lx:.1} {ly:.1}q10 -9 19 -2q-9 9 -19 2Z");
        }
    }
    (pot, cords, vines)
}

fn cat() -> [String; 5] {
    let x = 900.0;
    let y = DESK;
    [
        format!("M{} {}q6 -74 74 -76q74 -2 92 52q6 18 34 22q16 2 22 -2v10q-30 6 -44 -4q-16 -12 -22 -26q-18 -44 -80 -42q-62 2 -66 66Z", x - 108.0, y),
        format!("M{} {}q4 -70 72 -72q70 -2 88 50q8 22 40 24v8h-200Z", x - 108.0, y),
        format!("M{} {}q-10 -34 18 -44q2 -24 14 -6q16 -6 30 4q14 10 12 28q-4 26 -34 28q-32 2 -40 -10Z", x - 112.0, y - 52.0),
        format!("M{} {}l2 -26 20 16ZM{} {}l16 -22 6 26Z", x - 122.0, y - 88.0, x - 66.0, y - 96.0),
        format!("M{} {}q52 -4 66 -34q6 -12 -6 -15q-10 -3 -14 9q-9 20 -46 24Z", x + 76.0, y),
    ]
}

pub fn room(out: &mut String) {
    const INK: &str = "#05060a";
    let (frame, items) = shelf();
    let (pot, cords, vines) = hanger();
    let [body, fill, head, ears, tail] = cat();
    let d = DESK;
    let stack = format!("M300 {}h150v-16h-150ZM310 {}h134v-14h-134ZM296 {}h142v-13h-142Z", d, d - 16.0, d - 30.0);
    let cup = format!("M712 {}h58l-7 -70h-44Z", d);
    let pens = format!(
        "M722 {}l-8 -54h9l10 54ZM736 {}l2 -66h9l-2 66ZM752 {}l12 -48h9l-12 48Z",
        d - 70.0,
        d - 70.0,
        d - 70.0
    );
    let mug = format!("M604 {}h62l-6 -50h-50Z", d);
    let handle = format!("M666 {}q22 2 22 15q0 13 -20 15v-7q12 -1 12 -8q0 -7 -14 -8Z", d - 44.0);
    let steam = format!("M618 {}q-12 -18 2 -32q11 -13 2 -26M642 {}q-10 -15 2 -28q10 -11 2 -22", d - 60.0, d - 58.0);
    let bottle = format!("M1512 {}h44v-72h-12v-26h-20v26h-12Z", d);
    let desk = format!("M-40 {}h{}v{}h-{}Z", d, W + 80.0, H - d + 40.0, W + 80.0);

    for p in [&frame, &items, &pot, &vines, &stack, &body, &fill, &head, &ears, &tail, &desk, &cup, &pens, &mug, &handle, &bottle] {
        path(out, p, INK, "");
    }
    let _ = write!(out, "<path d=\"{cords}\" fill=\"none\" stroke=\"{INK}\" stroke-width=\"4\"/>");
    let _ = write!(
        out,
        "<path d=\"{steam}\" fill=\"none\" stroke=\"rgb(232,216,196)\" stroke-opacity=\"0.14\" stroke-width=\"4\" stroke-linecap=\"round\"/>"
    );
}

pub fn slice(width: f64, height: f64) -> String {
    let s = (width / W).max(height / H);
    let tx = (width - W * s) / 2.0;
    let ty = height - H * s;
    format!("translate({tx:.3} {ty:.3}) scale({s:.5})")
}

pub fn bottom_svg(width: u32, height: u32, sky_name: &str, view_name: &str, wet: f64) -> String {
    let (w, h) = (width as f64, height as f64);
    let s = sky(sky_name);
    let mut out = String::with_capacity(64 * 1024);
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><defs>\
         <linearGradient id=\"sky\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"{}\"/><stop offset=\"0.52\" stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient>\
         <filter id=\"soft\" x=\"-5%\" y=\"-5%\" width=\"110%\" height=\"110%\"><feGaussianBlur stdDeviation=\"1\"/></filter></defs>\
         <rect width=\"{w}\" height=\"{h}\" fill=\"url(#sky)\"/><g transform=\"{}\">",
        s[0],
        s[1],
        s[2],
        slice(w, h)
    );
    view(&mut out, view_name, wet);
    out.push_str("</g></svg>");
    out
}

pub fn top_svg(width: u32, height: u32, px: f64, wet: f64) -> String {
    let (w, h) = (width as f64, height as f64);
    let frame = (w * 0.06).min(76.0 * px);
    let mullion = (w * 0.014).min(18.0 * px);
    let sill = h * 0.18;
    let mut out = String::with_capacity(64 * 1024);
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><defs>\
         <linearGradient id=\"haze\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"rgb(150,170,200)\" stop-opacity=\"0.14\"/><stop offset=\"1\" stop-color=\"rgb(120,140,170)\" stop-opacity=\"0.05\"/></linearGradient>\
         <linearGradient id=\"sill\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"rgb(6,7,9)\" stop-opacity=\"0\"/><stop offset=\"0.62\" stop-color=\"rgb(6,7,9)\" stop-opacity=\"0.96\"/><stop offset=\"1\" stop-color=\"rgb(6,7,9)\" stop-opacity=\"0.96\"/></linearGradient>\
         </defs>"
    );
    let haze = wet * 0.85;
    if haze > 0.001 {
        let _ = write!(out, "<rect width=\"{w}\" height=\"{h}\" fill=\"url(#haze)\" opacity=\"{haze:.3}\"/>");
    }
    let band = "fill=\"rgb(9,10,13)\" fill-opacity=\"0.94\"";
    let _ = write!(
        out,
        "<path d=\"M0 0H{w}V{h}H0ZM{frame:.2} {frame:.2}V{:.2}H{:.2}V{frame:.2}Z\" fill-rule=\"evenodd\" {band}/>",
        h - frame,
        w - frame
    );
    let _ = write!(out, "<rect x=\"{:.2}\" y=\"0\" width=\"{mullion:.2}\" height=\"{h}\" {band}/>", w / 2.0 - mullion / 2.0);
    let _ = write!(out, "<rect x=\"0\" y=\"{:.2}\" width=\"{w}\" height=\"{mullion:.2}\" {band}/>", h * 0.46);
    let _ = write!(out, "<rect x=\"0\" y=\"{:.2}\" width=\"{w}\" height=\"{sill:.2}\" fill=\"url(#sill)\"/>", h - sill);
    let _ = write!(out, "<g transform=\"{}\">", slice(w, h));
    room(&mut out);
    out.push_str("</g></svg>");
    out
}
