// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Polygon stroking: a stroke is the union of one quad per segment, a wedge per join and a
//! cap shape per open end. Every piece is emitted with positive orientation, so filling them
//! together with the non-zero rule yields their exact union without computing offset curves.
//! Dashing splits the flattened polylines by arc length first.

use std::borrow::Cow;

use apeireth_image_doc::{LineCap, LineJoin};

use crate::flatten::Polyline;

/// Stroke geometry in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Miter limit as a multiple of the width (ratio miter length / width, like SVG).
    pub miter_limit: f64,
    /// Dash pattern in px (`on, off, on, off…`); empty = solid.
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        StrokeStyle { width: 1.0, cap: LineCap::Butt, join: LineJoin::Miter, miter_limit: 4.0, dashes: Vec::new(), dash_offset: 0.0 }
    }
}

type P = (f64, f64);

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}
fn add(a: P, b: P) -> P {
    (a.0 + b.0, a.1 + b.1)
}
fn mul(a: P, k: f64) -> P {
    (a.0 * k, a.1 * k)
}
fn len(a: P) -> f64 {
    a.0.hypot(a.1)
}
fn norm(a: P) -> P {
    let l = len(a);
    if l > 0.0 { (a.0 / l, a.1 / l) } else { (0.0, 0.0) }
}
/// Left normal in y-down coordinates.
fn perp(a: P) -> P {
    (a.1, -a.0)
}
fn cross(a: P, b: P) -> f64 {
    a.0 * b.1 - a.1 * b.0
}
fn dot(a: P, b: P) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn signed_area(p: &[P]) -> f64 {
    let n = p.len();
    (0..n).map(|i| cross(p[i], p[(i + 1) % n])).sum::<f64>() * 0.5
}

/// Pushes `poly` with positive orientation (drops degenerate pieces).
fn emit(out: &mut Vec<Vec<P>>, mut poly: Vec<P>) {
    let a = signed_area(&poly);
    if a.abs() < 1e-12 {
        return;
    }
    if a < 0.0 {
        poly.reverse();
    }
    out.push(poly);
}

/// Angular step so that arc chords deviate at most `tol` from a circle of radius `r`.
fn arc_step(r: f64, tol: f64) -> f64 {
    if r <= tol {
        return std::f64::consts::FRAC_PI_2;
    }
    (2.0 * (1.0 - tol / r).clamp(-1.0, 1.0).acos()).clamp(0.01, std::f64::consts::FRAC_PI_2)
}

/// Vertex radius for chords of `step` radians so each chord's pie slice has the area of the
/// circular sector (keeps round caps/joins area-exact at any tessellation).
fn area_radius(r: f64, step: f64) -> f64 {
    if step < 1e-9 { r } else { r * (step / step.sin()).sqrt() }
}

/// Points a full circle needs at `tol` (at least 8).
fn circle_points(r: f64, tol: f64) -> usize {
    ((std::f64::consts::TAU / arc_step(r, tol)).ceil() as usize).max(8)
}

/// A circle of at most `arc_cap` points (`usize::MAX` = no limit, at least 4).
fn circle(c: P, r: f64, tol: f64, arc_cap: usize) -> Vec<P> {
    let n = circle_points(r, tol).min(arc_cap).max(4);
    let rr = area_radius(r, std::f64::consts::TAU / n as f64);
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            (c.0 + rr * a.cos(), c.1 + rr * a.sin())
        })
        .collect()
}

/// Dash steps one stroke may spend, shared by all of its lines: it bounds the number of dashes
/// (and so of caps) for file-provided patterns. Past it, the rest is drawn solid.
const DASH_STEP_BUDGET: usize = 100_000;

/// Outline points a stroke may spend before its round caps and joins get coarser (#5211).
const OUTLINE_POINT_BUDGET: usize = 1 << 20;

/// Stroke outline pieces for `lines`. Fill them with the non-zero rule.
///
/// The outline is bounded: when full-resolution round caps and joins would exceed
/// [`OUTLINE_POINT_BUDGET`] points, every round cap and join gets the same, smaller number of
/// segments (down to 4 per circle and 1 per join; areas stay exact). Strokes under the budget
/// are unchanged and no piece is dropped, so the total is at most the larger of the budget and
/// that minimal tessellation, which is linear in the (budgeted) flattened points and dashes.
pub fn stroke_polygons(lines: &[Polyline], style: &StrokeStyle, tol: f64) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    if style.width <= 0.0 || !style.width.is_finite() || !style.miter_limit.is_finite() || style.miter_limit < 1.0 {
        return out;
    }
    let mut steps_left = DASH_STEP_BUDGET;
    let mut pieces: Vec<Cow<Polyline>> = Vec::new();
    for pl in lines {
        if pl.pts.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            continue;
        }
        if !style.dashes.is_empty() {
            pieces.extend(dash_polyline_budgeted(pl, &style.dashes, style.dash_offset, &mut steps_left).into_iter().map(Cow::Owned));
        } else {
            pieces.push(Cow::Borrowed(pl));
        }
    }
    let arc_cap = round_arc_cap(&pieces, style, tol);
    for pl in &pieces {
        stroke_one(pl, style, tol, arc_cap, &mut out);
    }
    out
}

/// Points per full circle allowed for round caps and joins so the outline of `pieces` stays
/// within [`OUTLINE_POINT_BUDGET`]; `usize::MAX` when full resolution fits.
fn round_arc_cap(pieces: &[Cow<Polyline>], style: &StrokeStyle, tol: f64) -> usize {
    let (round_cap, round_join) = (style.cap == LineCap::Round, style.join == LineJoin::Round);
    if !round_cap && !round_join {
        return usize::MAX;
    }
    // Upper bounds: ribbons and miter/bevel pieces, full circles (caps and dots), round joins.
    let (mut base, mut circles, mut joins) = (0usize, 0usize, 0usize);
    for pl in pieces {
        let m = pl.pts.len();
        if m == 0 {
            continue;
        }
        base = base.saturating_add(m.saturating_mul(6)).saturating_add(8);
        if round_cap && !(pl.closed && m > 2) {
            circles = circles.saturating_add(if m == 1 { 1 } else { 2 });
        }
        if round_join {
            joins = joins.saturating_add(m);
        }
    }
    // A join sweeps at most half a circle, plus its centre and two end points.
    let cost = |n: usize| base.saturating_add(circles.saturating_mul(n)).saturating_add(joins.saturating_mul(n / 2 + 3));
    let full = circle_points(style.width * 0.5, tol);
    if cost(full) <= OUTLINE_POINT_BUDGET {
        return usize::MAX;
    }
    let left = OUTLINE_POINT_BUDGET.saturating_sub(base).saturating_sub(joins.saturating_mul(3)) as f64;
    let per_circle = circles as f64 + joins as f64 * 0.5;
    ((left / per_circle.max(1.0)) as usize).clamp(4, full)
}

fn stroke_one(pl: &Polyline, style: &StrokeStyle, tol: f64, arc_cap: usize, out: &mut Vec<Vec<P>>) {
    let hw = style.width * 0.5;
    // Drop repeated points.
    let mut pts: Vec<P> = Vec::with_capacity(pl.pts.len());
    let mut knot: Vec<bool> = Vec::with_capacity(pl.pts.len());
    for (i, p) in pl.pts.iter().enumerate() {
        if pts.last().is_some_and(|q| len(sub(*p, *q)) < 1e-9) {
            continue;
        }
        pts.push(*p);
        knot.push(pl.knot.get(i).copied().unwrap_or(true));
    }
    let mut closed = pl.closed;
    if closed && pts.len() > 1 && len(sub(pts[0], pts[pts.len() - 1])) < 1e-9 {
        pts.pop();
        knot.pop();
    }
    if pts.len() < 3 {
        closed = false;
    }
    if pts.is_empty() {
        return;
    }
    if pts.len() == 1 {
        // A dot: only round and square caps draw something.
        let c = pts[0];
        match style.cap {
            LineCap::Round => emit(out, circle(c, hw, tol, arc_cap)),
            LineCap::Square => emit(out, vec![(c.0 - hw, c.1 - hw), (c.0 + hw, c.1 - hw), (c.0 + hw, c.1 + hw), (c.0 - hw, c.1 + hw)]),
            LineCap::Butt => {}
        }
        return;
    }
    let n = pts.len();
    let segs = if closed { n } else { n - 1 };
    let dir = |i: usize| norm(sub(pts[(i + 1) % n], pts[i]));
    let seg_len = |i: usize| len(sub(pts[(i + 1) % n], pts[i]));
    // A vertex continues a ribbon (no separate join piece) when it is a flattening point with a
    // small turn whose inner offset does not fold back, or a knot where the path is straight.
    let cos_smooth = 25f64.to_radians().cos();
    let smooth = |j: usize| -> bool {
        if j == 0 || (!closed && j == n - 1) {
            return false;
        }
        let (i0, i1) = ((j + n - 1) % n, j);
        let d = dot(dir(i0), dir(i1));
        if knot[j] {
            return d > 1.0 - 1e-12;
        }
        if d < cos_smooth {
            return false;
        }
        let tan_half = ((1.0 - d) / (1.0 + d)).max(0.0).sqrt();
        hw * tan_half < 0.5 * seg_len(i0).min(seg_len(i1))
    };
    let mut left: Vec<P> = Vec::new();
    let mut right: Vec<P> = Vec::new();
    for i in 0..segs {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let nrm = mul(perp(dir(i)), hw);
        if left.is_empty() {
            left.push(add(a, nrm));
            right.push(sub(a, nrm));
        }
        let j = (i + 1) % n;
        if i + 1 < segs + usize::from(!closed) && smooth(j) {
            // Offset lines of both segments meet at v ± bisector · hw / cos(θ/2).
            let n2 = mul(perp(dir(j)), hw);
            let bis = norm(add(nrm, n2));
            let cos_half = ((1.0 + dot(dir(i), dir(j))) * 0.5).max(1e-9).sqrt();
            let off = mul(bis, hw / cos_half);
            left.push(add(b, off));
            right.push(sub(b, off));
            continue;
        }
        left.push(add(b, nrm));
        right.push(sub(b, nrm));
        let mut poly = std::mem::take(&mut left);
        poly.extend(right.drain(..).rev());
        emit(out, poly);
        let is_join = closed || j != n - 1;
        if is_join {
            let prev = pts[i];
            let v = pts[j];
            let next = pts[(j + 1) % n];
            let d1 = norm(sub(v, prev));
            let d2 = norm(sub(next, v));
            // Flattening points use miters (the true offset of a smooth curve) unless the
            // requested join is round; bevels only at real corners.
            let join = match (style.join, knot[j]) {
                (LineJoin::Round, _) => LineJoin::Round,
                (jn, true) => jn,
                (_, false) => LineJoin::Miter,
            };
            join_piece(v, d1, d2, hw, join, if knot[j] { style.miter_limit.min(500.0) } else { f64::INFINITY }, tol, arc_cap, out);
        }
    }
    if !closed {
        let d0 = norm(sub(pts[1], pts[0]));
        let d1 = norm(sub(pts[n - 1], pts[n - 2]));
        cap_piece(pts[0], mul(d0, -1.0), hw, style.cap, tol, arc_cap, out);
        cap_piece(pts[n - 1], d1, hw, style.cap, tol, arc_cap, out);
    }
}

#[allow(clippy::too_many_arguments)]
fn join_piece(v: P, d1: P, d2: P, hw: f64, join: LineJoin, miter_limit: f64, tol: f64, arc_cap: usize, out: &mut Vec<Vec<P>>) {
    let c = cross(d1, d2);
    let d = dot(d1, d2);
    if c.abs() < 1e-12 && d > 0.0 {
        return;
    }
    // Outer side: a right turn (c > 0 in y-down) opens the left side.
    let s = if c > 0.0 { 1.0 } else { -1.0 };
    let n1 = mul(perp(d1), hw * s);
    let n2 = mul(perp(d2), hw * s);
    let (a, b) = (add(v, n1), add(v, n2));
    match join {
        LineJoin::Bevel => emit(out, vec![v, a, b]),
        LineJoin::Miter => {
            // Miter length / width = 1 / sin(θ/2) where θ is the angle between the segments,
            // i.e. 1 / cos of half the turning angle.
            let cos_half = ((1.0 + d) * 0.5).max(0.0).sqrt();
            if cos_half < 1e-9 || 1.0 / cos_half > miter_limit {
                emit(out, vec![v, a, b]);
            } else {
                let bis = norm(add(n1, n2));
                let m = add(v, mul(bis, hw / cos_half));
                emit(out, vec![v, a, m, b]);
            }
        }
        LineJoin::Round => {
            let a0 = n1.1.atan2(n1.0);
            let mut a1 = n2.1.atan2(n2.0);
            // Sweep the short way from n1 to n2 on the outer side.
            let mut delta = a1 - a0;
            while delta > std::f64::consts::PI {
                delta -= std::f64::consts::TAU;
            }
            while delta < -std::f64::consts::PI {
                delta += std::f64::consts::TAU;
            }
            if c.abs() < 1e-12 {
                // U-turn: half circle on the side of n1 rotating towards d1.
                delta = std::f64::consts::PI * if cross(n1, d1) > 0.0 { 1.0 } else { -1.0 };
            }
            a1 = a0 + delta;
            let mut steps = (delta.abs() / arc_step(hw, tol)).ceil() as usize;
            if arc_cap != usize::MAX {
                // Over the outline budget: the join's share of an `arc_cap`-point circle.
                steps = steps.min((arc_cap as f64 * delta.abs() / std::f64::consts::TAU).ceil() as usize);
            }
            let steps = steps.max(1);
            let rr = area_radius(hw, delta.abs() / steps as f64);
            let mut poly = vec![v, a];
            for k in 1..steps {
                let t = a0 + (a1 - a0) * k as f64 / steps as f64;
                poly.push((v.0 + rr * t.cos(), v.1 + rr * t.sin()));
            }
            poly.push(b);
            emit(out, poly);
        }
    }
}

fn cap_piece(p: P, dir: P, hw: f64, cap: LineCap, tol: f64, arc_cap: usize, out: &mut Vec<Vec<P>>) {
    match cap {
        LineCap::Butt => {}
        LineCap::Round => emit(out, circle(p, hw, tol, arc_cap)),
        LineCap::Square => {
            let n = mul(perp(dir), hw);
            let e = mul(dir, hw);
            emit(out, vec![add(p, n), add(add(p, n), e), add(sub(p, n), e), sub(p, n)]);
        }
    }
}

/// Splits a polyline into dashes (open polylines). `pattern` alternates on/off lengths in px.
pub fn dash_polyline(pl: &Polyline, pattern: &[f64], offset: f64) -> Vec<Polyline> {
    dash_polyline_budgeted(pl, pattern, offset, &mut DASH_STEP_BUDGET.clone())
}

/// [`dash_polyline`] spending at most `steps_left` dash steps (decremented); once they are
/// spent, the rest of the line is drawn solid.
fn dash_polyline_budgeted(pl: &Polyline, pattern: &[f64], offset: f64, steps_left: &mut usize) -> Vec<Polyline> {
    if pattern.is_empty() {
        return vec![pl.clone()];
    }
    if pattern.len() > 32
        || !offset.is_finite()
        || pattern.iter().any(|d| !d.is_finite() || *d < 0.0)
        || pl.pts.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Vec::new();
    }
    let mut pat = pattern.to_vec();
    if pat.len() % 2 == 1 {
        let copy = pat.clone();
        pat.extend(copy);
    }
    let total: f64 = pat.iter().sum();
    if !total.is_finite() || total <= 1e-9 || pl.pts.len() < 2 {
        return Vec::new();
    }
    // Only a closed polyline needs a copy (with its first point repeated at the end).
    let pts: Cow<[P]> = if pl.closed {
        let mut pts = pl.pts.clone();
        pts.push(pl.pts[0]);
        pts.into()
    } else {
        pl.pts.as_slice().into()
    };
    // Knot flags past the source's (and the repeated closing point) count as knots.
    let knot_at = |i: usize| i >= pl.pts.len() || pl.knot.get(i).copied().unwrap_or(true);
    // Position in the pattern.
    let mut idx = 0usize;
    let mut left = pat[0];
    let mut skip = offset.rem_euclid(total);
    while skip > 0.0 {
        if skip >= left {
            skip -= left;
            idx = (idx + 1) % pat.len();
            left = pat[idx];
        } else {
            left -= skip;
            skip = 0.0;
        }
    }
    let mut out = Vec::new();
    let mut cur: Option<Polyline> = if idx.is_multiple_of(2) { Some(Polyline { pts: vec![pts[0]], knot: vec![true], closed: false }) } else { None };
    'walk: for (wi, w) in pts.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let b_knot = knot_at(wi + 1);
        let seg = len(sub(b, a));
        let mut pos = 0.0;
        while seg - pos > 1e-12 {
            let step = left.min(seg - pos);
            if *steps_left == 0 || (step > 0.0 && pos + step == pos) {
                // Past the step budget (which bounds allocations and time for file-provided
                // patterns; commands reject such patterns before mutating the document) or
                // dashes below the coordinates' precision: keep the dashes made so far and draw
                // the rest of the line solid rather than dropping the whole stroke (#5195).
                let here = if pos > 0.0 { add(a, mul(sub(b, a), pos / seg)) } else { a };
                let rest = cur.get_or_insert_with(|| Polyline { pts: vec![here], knot: vec![true], closed: false });
                for (j, p) in pts.iter().enumerate().skip(wi + 1) {
                    rest.pts.push(*p);
                    rest.knot.push(knot_at(j));
                }
                break 'walk;
            }
            *steps_left -= 1;
            pos += step;
            left -= step;
            let at_end = seg - pos <= 1e-12;
            let p = if at_end { b } else { add(a, mul(sub(b, a), pos / seg)) };
            if let Some(c) = cur.as_mut() {
                c.pts.push(p);
                c.knot.push(!at_end || b_knot);
            }
            if left <= 1e-12 {
                idx = (idx + 1) % pat.len();
                left = pat[idx];
                if idx % 2 == 1 {
                    if let Some(c) = cur.take() {
                        out.push(c);
                    }
                } else {
                    cur = Some(Polyline { pts: vec![p], knot: vec![true], closed: false });
                }
            }
        }
        // The vertex at `b` is a join inside a dash: keep it (already pushed when reached).
    }
    if let Some(c) = cur.take()
        && c.pts.len() > 1
    {
        out.push(c);
    }
    if pl.closed {
        // A dash that crosses the closing vertex has a join there, not two caps.
        // Preserve knot flags when stitching the tail onto the first dash.
        let seam = pl.pts.first().copied();
        let wraps = out.first().and_then(|d| d.pts.first()).copied() == seam && out.last().and_then(|d| d.pts.last()).copied() == seam;
        if wraps && out.len() == 1 {
            if let Some(dash) = out.first_mut() {
                dash.closed = true;
            }
        } else if wraps
            && out.len() > 1
            && let Some(mut tail) = out.pop()
            && let Some(first) = out.first_mut()
        {
            tail.pts.extend(first.pts.iter().skip(1).copied());
            tail.knot.extend(first.knot.iter().skip(1).copied());
            *first = tail;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(pts: &[P], closed: bool) -> Polyline {
        Polyline { pts: pts.to_vec(), knot: vec![true; pts.len()], closed }
    }

    #[test]
    fn dashes_split_by_length() {
        let pl = line(&[(0.0, 0.0), (10.0, 0.0)], false);
        let d = dash_polyline(&pl, &[2.0, 1.0], 0.0);
        // on [0,2] [3,5] [6,8] [9,10]
        assert_eq!(d.len(), 4);
        assert!((d[1].pts[0].0 - 3.0).abs() < 1e-9 && (d[1].pts.last().unwrap().0 - 5.0).abs() < 1e-9);
        assert!((d[3].pts.last().unwrap().0 - 10.0).abs() < 1e-9);
        let o = dash_polyline(&pl, &[2.0, 1.0], 1.0);
        assert!((o[0].pts.last().unwrap().0 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pieces_are_positive() {
        let pl = line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false);
        for join in [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel] {
            for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
                let st = StrokeStyle { width: 2.0, join, cap, ..Default::default() };
                for p in stroke_polygons(std::slice::from_ref(&pl), &st, 0.1) {
                    assert!(signed_area(&p) > 0.0);
                }
            }
        }
    }

    #[test]
    fn closed_dashes_have_a_join_at_the_seam() {
        let square = line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], true);
        let dashes = dash_polyline(&square, &[8.0, 4.0], 2.0);
        assert_eq!(dashes.len(), 3);
        let crossing = &dashes[0];
        assert_eq!(crossing.pts, [(0.0, 6.0), (0.0, 0.0), (6.0, 0.0)]);
        assert_eq!(crossing.knot, [true, true, true]);
        let full = dash_polyline(&square, &[80.0, 2.0], 0.0);
        assert_eq!(full.len(), 1);
        assert!(full[0].closed);
    }

    #[test]
    fn invalid_and_too_dense_dashes_finish_with_bounded_work() {
        let pl = line(&[(0.0, 0.0), (100.0, 0.0)], false);
        for pattern in [vec![0.0, 0.0], vec![f64::INFINITY, 1.0], vec![f64::NAN, 1.0], vec![-1.0, 2.0], vec![1.0; 33]] {
            assert!(dash_polyline(&pl, &pattern, 0.0).is_empty());
        }
        assert!(dash_polyline(&pl, &[2.0, 1.0], f64::INFINITY).is_empty());
        assert!(dash_polyline(&line(&[(f64::NAN, 0.0), (1.0, 0.0)], false), &[2.0, 1.0], 0.0).is_empty());
        // Too dense for the step budget: bounded work, and the line is still drawn.
        let d = dash_polyline(&pl, &[1e-8, 1e-8], 0.0);
        assert!(!d.is_empty() && d.len() <= 50_001);
        assert_eq!(d.last().and_then(|p| p.pts.last()), Some(&(100.0, 0.0)));
    }

    #[test]
    fn dashes_past_the_step_budget_keep_the_rest_of_the_line_solid() {
        // An SVG `stroke-dasharray="0.5 0.5"` along 60000 px needs more steps than the budget:
        // the stroke used to vanish entirely (#5195). The dashes made so far stay, the rest of
        // the line (through its later vertices) is drawn solid.
        let pl = line(&[(0.0, 0.0), (60_000.0, 0.0), (60_000.0, 1000.0)], false);
        let d = dash_polyline(&pl, &[0.5, 0.5], 0.0);
        assert!(d.len() > 40_000, "{} dashes", d.len());
        assert_eq!(d[0].pts, [(0.0, 0.0), (0.5, 0.0)]);
        let tail = d.last().unwrap();
        assert_eq!(tail.pts.last(), Some(&(60_000.0, 1000.0)));
        assert!(tail.pts.contains(&(60_000.0, 0.0)) && tail.pts.len() == tail.knot.len());
        // The solid tail starts where the dashes stopped (100000 steps of 0.5 px).
        assert!((tail.pts[0].0 - 50_000.0).abs() < 1.0 && tail.pts[0].1 == 0.0, "{:?}", tail.pts[0]);
        // Closed outlines finish the same way, back to their first point, where the solid tail
        // joins the first dash (one piece with a join at the seam, no caps).
        let outline = line(&[(0.0, 0.0), (60_000.0, 0.0), (60_000.0, 1000.0), (0.0, 1000.0)], true);
        let d = dash_polyline(&outline, &[0.5, 0.5], 0.0);
        let joined = &d[0];
        assert!(joined.pts.contains(&(60_000.0, 1000.0)) && joined.pts.contains(&(0.0, 1000.0)) && joined.pts.contains(&(0.0, 0.0)));
        assert_eq!(joined.pts.last(), Some(&(0.5, 0.0)));
    }

    #[test]
    fn wide_round_capped_dashes_keep_the_outline_within_its_point_budget() {
        let points = |polys: &[Vec<P>]| polys.iter().map(Vec::len).sum::<usize>();
        // 5000 dashes, each with two round caps of ~629 points at full resolution (#5211).
        let st = StrokeStyle { width: 4000.0, cap: LineCap::Round, join: LineJoin::Round, dashes: vec![1.0, 9.0], ..Default::default() };
        let pl = line(&[(0.0, 0.0), (25_000.0, 0.0), (25_000.0, 25_000.0)], false);
        let polys = stroke_polygons(std::slice::from_ref(&pl), &st, 0.01);
        assert!(points(&polys) <= OUTLINE_POINT_BUDGET, "{} points", points(&polys));
        // No dash is dropped: a ribbon and two caps each.
        assert!(polys.len() >= 3 * 5000, "{} pieces", polys.len());
        // Many tiny undashed subpaths: bounded too, every one still drawn.
        let dots: Vec<Polyline> = (0..3000).map(|i| line(&[(f64::from(i), 0.0), (f64::from(i) + 0.5, 0.0)], false)).collect();
        let polys = stroke_polygons(&dots, &StrokeStyle { dashes: Vec::new(), ..st.clone() }, 0.01);
        assert!(points(&polys) <= OUTLINE_POINT_BUDGET && polys.len() >= 3 * 3000, "{} points", points(&polys));
        // Under the budget, round caps keep their full resolution.
        let one = stroke_polygons(&dots[..1], &StrokeStyle { dashes: Vec::new(), ..st }, 0.01);
        assert!(one.iter().any(|p| p.len() == circle_points(2000.0, 0.01)) && circle_points(2000.0, 0.01) > 600);
    }

    #[test]
    fn nonuniform_patterns_and_negative_offsets_repeat_by_arc_length() {
        let pl = line(&[(0.0, 0.0), (30.0, 0.0)], false);
        let d = dash_polyline(&pl, &[4.0, 2.0, 1.0, 3.0], -10.0);
        assert_eq!(
            d.iter().map(|p| (p.pts.first().unwrap().0, p.pts.last().unwrap().0)).collect::<Vec<_>>(),
            [(0.0, 4.0), (6.0, 7.0), (10.0, 14.0), (16.0, 17.0), (20.0, 24.0), (26.0, 27.0)]
        );
    }
}
