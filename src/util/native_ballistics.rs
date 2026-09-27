//! Shared native ballistic speed used by the Rules postpass and shot launch.

use super::native_x87::{NativeF64Bits, X87Chop53 as X};

/// `Ballistic_Launch_Speed @ 0x0048AB90`, including the callers' Floater
/// gravity store (`0x0048ACF0`). Inputs are signed leptons and Rules Gravity.
///
/// Reuses the deterministic arithmetic already required by FireAt: native
/// stores, approximate square root and truncation can change flight/contact.
/// Execution corpora: `tools/projectile_oracle/fireat_speed.json` and
/// `tools/rules_oracle/weapon_speed_order.json`.
pub(crate) fn ballistic_launch_speed(distance: i32, gravity: i32, floater: bool) -> i32 {
    let mut gravity = X::load_i32(gravity);
    if floater {
        gravity = X::mul(gravity, X::load_f64(NativeF64Bits::HALF).unwrap());
    }
    let gravity = X::load_f64(X::store_f64(gravity).unwrap()).unwrap();
    let product = X::mul(
        X::mul(X::load_i32(distance), gravity),
        X::load_f64(NativeF64Bits::from_bits(0x3ff3_3333_3333_3333)).unwrap(),
    );
    let stored = X::load_f64(X::store_f64(product).unwrap()).unwrap();
    let root = super::native_x87::sqrt_approx_f32(stored)
        .expect("signed32 ballistic inputs fit the native square-root lookup domain");
    X::ftol_i64(X::load_f32(root).unwrap())
        .expect("ballistic speed fits the native signed64 conversion") as i32
}
