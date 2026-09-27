//! Shared native projectile scalar operations and lookup boundaries.
//! FireAt (6FE8EE), HomingTrack (5B20F0), and their direction helpers use
//! the same represented arithmetic. Keep stores explicit at their callers.

use crate::map::retail_trig::TrigTable;
use crate::util::native_x87::{
    NativeF32Bits, NativeF64Bits, X87Chop53 as X, X87Ordering, X87Value,
};

pub(super) const PI_HALF: NativeF64Bits = NativeF64Bits::from_bits(0x3ff9_21fb_5444_2d18);
const WORD_SCALE: NativeF64Bits = NativeF64Bits::from_bits(0xc0c4_5f07_af68_ecef);
const RADIAN_SCALE: NativeF64Bits = NativeF64Bits::from_bits(0xbf19_222d_989f_5e57);
const TRIG_SCALE: NativeF32Bits = NativeF32Bits::from_bits(0x4522_f983);

pub(super) fn d(bits: NativeF64Bits) -> X87Value {
    X::load_f64(bits).expect("finite native launch input")
}

pub(super) fn f(bits: NativeF32Bits) -> X87Value {
    X::load_f32(bits).expect("finite native launch table value")
}

pub(super) fn store(value: X87Value) -> NativeF64Bits {
    X::store_f64(value).expect("finite stored native launch value")
}

pub(super) fn round(value: X87Value) -> X87Value {
    d(store(value))
}

pub(super) fn int(value: X87Value) -> i32 {
    X::ftol_i64(value).expect("native launch conversion fits signed i64") as i32
}

pub(super) fn sqrt(value: X87Value) -> X87Value {
    f(crate::util::native_x87::sqrt_approx_f32(round(value))
        .expect("native launch squared value fits the finite lookup domain"))
}

pub(super) fn less(a: X87Value, b: X87Value) -> bool {
    X::compare(a, b) == X87Ordering::Less
}

pub(super) fn radians(word: u16) -> NativeF64Bits {
    store(X::mul(
        X::load_i32(i32::from(word as i16) - 0x3fff),
        d(RADIAN_SCALE),
    ))
}

pub(super) fn angle_word(angle: X87Value) -> u16 {
    int(X::mul(X::sub(angle, d(PI_HALF)), d(WORD_SCALE))) as u16
}

pub(super) fn sin(table: &TrigTable, angle: NativeF64Bits) -> X87Value {
    let units = int(X::mul(d(angle), f(TRIG_SCALE)));
    f(NativeF32Bits::from_bits(table.sin(units).to_bits()))
}

pub(super) fn cos(table: &TrigTable, angle: NativeF64Bits) -> X87Value {
    let units = int(X::mul(d(angle), f(TRIG_SCALE)));
    f(NativeF32Bits::from_bits(table.cos(units).to_bits()))
}

pub(super) fn atan(y: X87Value, x: X87Value) -> X87Value {
    crate::util::direction_tables::native_atan2_f32(
        X::store_f32(y).expect("finite launch atan numerator"),
        X::store_f32(x).expect("finite launch atan denominator"),
    )
}
