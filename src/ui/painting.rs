use crate::theme::Theme;
use gpui::*;

/// Paint the outside of a continuous corner, including over cover images.
pub fn continuous_corner_mask(radius: f32, color: Hsla) -> impl IntoElement {
    canvas(|bounds, _, _| bounds, move |_, b, window, _| {
        let x = f32::from(b.left()); let y = f32::from(b.top());
        let w = f32::from(b.size.width); let h = f32::from(b.size.height);
        let r = radius.min(w / 2.).min(h / 2.);
        for (sx, sy, ux, uy, vx, vy) in [
            (x + w - r, y, 1., 0., 0., 1.),
            (x + w, y + h - r, 0., 1., -1., 0.),
            (x + r, y + h, -1., 0., 0., -1.),
            (x, y + r, 0., -1., 1., 0.),
        ] {
            let q = |u: f32, v: f32| point(px(sx + r * (u * ux + v * vx)), px(sy + r * (u * uy + v * vy)));
            let mut path = PathBuilder::fill();
            path.move_to(q(0., 0.));
            path.line_to(q(1., 0.));
            path.line_to(q(1., 1.));
            path.cubic_bezier_to(q(std::f32::consts::FRAC_1_SQRT_2, 0.2928932), q(1., 0.7928932), q(1., 0.5857864));
            path.cubic_bezier_to(q(0., 0.), q(0.4142136, 0.), q(0.2071068, 0.));
            path.close();
            if let Ok(path) = path.build() { window.paint_path(path, color); }
        }
    }).absolute().inset_0()
}

/// A capsule filled to `fraction` from the left, its edge a moving wave (the
/// update pill's download progress). Redraws every frame while shown.
pub fn wave_fill(fraction: f32, color: Hsla) -> impl IntoElement {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let t = START.get_or_init(std::time::Instant::now).elapsed().as_secs_f32();
    canvas(|bounds, _, _| bounds, move |_, b, window, _| {
        window.request_animation_frame();
        let f = fraction.clamp(0., 1.);
        if f <= 0. { return }
        let (x, y) = (f32::from(b.left()), f32::from(b.top()));
        let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
        let r = h / 2.;
        // The capsule's left and right walls at height v (0 = top).
        let wall = |v: f32| (r * r - (v - r).powi(2)).max(0.).sqrt();
        // A 2pt swell, one crest per pill height, drifting downwards; flat when full.
        let amp = if f >= 1. { 0. } else { 2. };
        let edge = |v: f32| (x + f * w + amp * (std::f32::consts::TAU * v / h - t * 5.).sin()).clamp(x + r - wall(v), x + w - r + wall(v));
        const N: usize = 24;
        let mut path = PathBuilder::fill();
        path.move_to(point(px(edge(0.)), px(y)));
        for i in 1..=N { let v = h * i as f32 / N as f32; path.line_to(point(px(edge(v)), px(y + v))); }
        for i in (0..=N).rev() { let v = h * i as f32 / N as f32; path.line_to(point(px(x + r - wall(v)), px(y + v))); }
        path.close();
        if let Ok(path) = path.build() { window.paint_path(path, color); }
    }).absolute().inset_0()
}

pub fn continuous_page(radius: f32) -> Div { continuous_surface(radius, Theme::bg_body()) }

pub fn continuous_surface(radius: f32, color: Hsla) -> Div {
    div().relative().child(canvas(|bounds, _, _| bounds, move |_, bounds, window, _| {
        let inset = 0.;
                let x = f32::from(bounds.left()) + inset;
                let y = f32::from(bounds.top()) + inset;
                let w = f32::from(bounds.size.width) - inset * 2.;
                let h = f32::from(bounds.size.height) - inset * 2.;
                let r = (radius - inset).max(0.).min(w / 2.).min(h / 2.);
                let pt = |a, b| point(px(a), px(b));
                let mut path = PathBuilder::fill();
                // Two cubics per corner: zero curvature at the straight joins,
                // a shared tangent/curvature at the midpoint, and a circular-size
                // diagonal inset rather than the previous overly square curve.
                let corner = |path: &mut PathBuilder, sx: f32, sy: f32, ux: f32, uy: f32, vx: f32, vy: f32| {
                    let q = |u: f32, v: f32| pt(sx + r * (u * ux + v * vx), sy + r * (u * uy + v * vy));
                    path.cubic_bezier_to(q(std::f32::consts::FRAC_1_SQRT_2, 0.2928932), q(0.2071068, 0.), q(0.4142136, 0.));
                    path.cubic_bezier_to(q(1., 1.), q(1., 0.5857864), q(1., 0.7928932));
                };
                path.move_to(pt(x + r, y));
                path.line_to(pt(x + w - r, y));
                corner(&mut path, x + w - r, y, 1., 0., 0., 1.);
                path.line_to(pt(x + w, y + h - r));
                corner(&mut path, x + w, y + h - r, 0., 1., -1., 0.);
                path.line_to(pt(x + r, y + h));
                corner(&mut path, x + r, y + h, -1., 0., 0., -1.);
                path.line_to(pt(x, y + r));
                corner(&mut path, x, y + r, 0., -1., 1., 0.);
                path.close();
                if let Ok(path) = path.build() { window.paint_path(path, color); }
    }).absolute().inset_0())
}
