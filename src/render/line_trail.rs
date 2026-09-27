//! Original LineTrail556C00 and Surface4BEAC0 ordered pixel producer.
//!
//! The integer three-axis walk includes repeated destination pixels when Z
//! dominates. Its output is consumed in order against the shared live Z and
//! RGB565 destination; it never writes Z. Native goldens: line_trail.json.

use crate::sim::projectile::ProjectileCoord;
use crate::util::native_x87::adjust_for_z_standard;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LineTrailSegment {
    pub from: ProjectileCoord,
    pub to: ProjectileCoord,
    pub color: [u8; 3],
    pub strength: i32,
}

/// Logical pixels are unzoomed screen coordinates; ZOrigin remains an explicit
/// native input and never becomes a second depth buffer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineTrailViewport {
    pub camera: [i32; 2],
    pub clip: [i32; 4],
    pub z_origin_y: i32,
    pub zoom: f32,
}

/// The actual Surface4BEAC0 boundary: points relative to the clip origin,
/// signed endpoint adjustments and one constant intensity for this segment.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedLine {
    pub from: [i32; 2],
    pub to: [i32; 2],
    pub z_adjust: [i32; 2],
    pub strength: i32,
}

impl LineTrailSegment {
    /// VERA's absolute pixel frame carries the same +15 row bias as all world
    /// layers. Native556C00 passes the camera-relative projection unchanged;
    /// Surface4BEAC0 adds the clip origin. No motion interpolation enters history.
    pub(crate) fn project(self, camera: [i32; 2]) -> ProjectedLine {
        let project = |coord: ProjectileCoord| {
            let (x, y) = crate::util::lepton::absolute_leptons_to_screen(coord.x, coord.y, coord.z);
            [x as i32 - camera[0], y as i32 - camera[1]]
        };
        ProjectedLine {
            from: project(self.from),
            to: project(self.to),
            z_adjust: [
                (-2i32).wrapping_sub(adjust_for_z_standard(self.from.z)),
                (-2i32).wrapping_sub(adjust_for_z_standard(self.to.z)),
            ],
            strength: self.strength,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinePixel {
    pub point: [i32; 2],
    pub z: u16,
}

/// Original4BEAC0 ->7BC2B0 clips XY first, then interpolates the endpoint Z
/// adjustments using truncated integer lengths (4C1B50). Presentation f64 is
/// bounded by native clipped controls, not a simulation arithmetic authority.
pub(crate) fn rasterize(
    line: ProjectedLine,
    clip: [i32; 4],
    z_origin_y: i32,
    mut emit: impl FnMut(LinePixel),
) {
    if line.strength < 8 || clip[2] <= 0 || clip[3] <= 0 {
        return;
    }
    let mut a = [
        line.from[0].wrapping_add(clip[0]),
        line.from[1].wrapping_add(clip[1]),
    ];
    let mut b = [
        line.to[0].wrapping_add(clip[0]),
        line.to[1].wrapping_add(clip[1]),
    ];
    let mut za = line.z_adjust[0];
    let mut zb = line.z_adjust[1];
    if a[0] > b[0] {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut za, &mut zb);
    }
    let original_a = a;
    let original_b = b;
    if !clip_line(&mut a, &mut b, clip) {
        return;
    }
    // Original4C1B50 stores x²+y² then uses the existing retail table owner,
    // not host sqrt. Its two-coordinate scalar equals the zero-Z helper.
    let length = |a: [i32; 2], b: [i32; 2]| {
        crate::util::native_x87::distance_3d_leptons([a[0], a[1], 0], [b[0], b[1], 0])
    };
    let full_length = if a != original_a || b != original_b {
        length(original_a, original_b)
    } else {
        0
    };
    let clipped_adjust = |distance: i32, delta: i32| {
        use crate::util::native_x87::X87Chop53 as X;
        let ratio = X::div(X::load_i32(distance), X::load_i32(full_length))
            .expect("nonzero clipped line length");
        X::ftol_i64(X::mul(ratio, X::load_i32(delta))).expect("finite clipped line adjustment")
            as i32
    };
    let original_za = za;
    let original_zb = zb;
    if b != original_b {
        let delta = clipped_adjust(length(original_a, b), original_za.wrapping_sub(original_zb));
        zb = if original_za < original_zb {
            original_za.wrapping_add(delta.wrapping_abs())
        } else {
            original_za.wrapping_sub(delta.wrapping_abs())
        };
    }
    if a != original_a {
        let delta = clipped_adjust(length(a, original_b), original_za.wrapping_sub(original_zb));
        za = if original_zb > original_za {
            original_zb.wrapping_sub(delta.wrapping_abs())
        } else {
            original_zb.wrapping_add(delta.wrapping_abs())
        };
    }
    let dx = b[0] - a[0];
    let dy = (b[1] - a[1]).abs();
    let dz = zb.wrapping_sub(za).wrapping_abs();
    let y_step = if b[1] < a[1] { -1 } else { 1 };
    let z_step = if zb < za { -1 } else { 1 };
    let dominant = if dz > dx && dz > dy {
        2
    } else if dx > dy {
        0
    } else {
        1
    };
    let lengths = [dx, dy, dz];
    let count = lengths[dominant];
    let mut errors = [-count; 3];
    let mut point = a;
    let mut adjustment = za;
    for _ in 0..count {
        for axis in 0..3 {
            if axis != dominant {
                errors[axis] += 2 * lengths[axis];
            }
        }
        // 4BEF03/4BF186/4BF3EE narrow before unsigned comparison, unlike SHP.
        let z = (crate::render::native_z::DEFAULT_Z.wrapping_add(z_origin_y) as u16 as i32)
            .wrapping_sub(point[1])
            .wrapping_sub(clip[1])
            .wrapping_add(adjustment) as u16;
        emit(LinePixel { point, z });
        for axis in 0..3 {
            if axis == dominant || errors[axis] > 0 {
                match axis {
                    0 => point[0] += 1,
                    1 => point[1] += y_step,
                    _ => adjustment += z_step,
                }
                if axis != dominant {
                    errors[axis] -= 2 * count;
                }
            }
        }
    }
}

fn clip_line(a: &mut [i32; 2], b: &mut [i32; 2], clip: [i32; 4]) -> bool {
    let left = f64::from(clip[0]);
    let top = f64::from(clip[1]);
    let right = f64::from(clip[0] + clip[2]);
    let bottom = f64::from(clip[1] + clip[3]);
    let mut p = a.map(f64::from);
    let mut q = b.map(f64::from);
    let xy = (q[0] - p[0]) / (q[1] - p[1]);
    let yx = (q[1] - p[1]) / (q[0] - p[0]);
    let code = |v: [f64; 2]| -> u8 {
        (if v[0] < left {
            1
        } else if v[0] >= right {
            2
        } else {
            0
        }) | (if v[1] < top {
            8
        } else if v[1] >= bottom {
            4
        } else {
            0
        })
    };
    loop {
        let pc = code(p);
        let qc = code(q);
        if pc | qc == 0 {
            *a = p.map(|v| v as i32);
            *b = q.map(|v| v as i32);
            return true;
        }
        if pc & qc != 0 {
            return false;
        }
        let c = if pc != 0 { pc } else { qc };
        let v = if c & 8 != 0 {
            [(top - p[1]) * xy + p[0], top]
        } else if c & 4 != 0 {
            [(bottom - 1.0 - p[1]) * xy + p[0], bottom - 1.0]
        } else if c & 2 != 0 {
            [right - 1.0, (right - 1.0 - p[0]) * yx + p[1]]
        } else {
            [left, (left - p[0]) * yx + p[1]]
        };
        if c == pc {
            p = v;
        } else {
            q = v;
        }
    }
}

#[cfg(test)]
#[path = "line_trail_tests.rs"]
mod tests;
