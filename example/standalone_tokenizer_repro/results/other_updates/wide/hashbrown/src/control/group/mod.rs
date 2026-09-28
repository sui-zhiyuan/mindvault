// TESTING NOTE:
//
// Because this module uses `cfg(..)` to select an implementation, it will not
// be linted without being run on targets that actually load each of these
// modules. Be sure to edit `ci/tools.sh` to add in the necessary cfgs if you
// change these, so that your implementation gets properly linted.

#[cfg(all(any(hb_backend = "neon16", hb_backend = "sve16", hb_backend = "sve32", hb_backend = "sve16r", hb_backend = "sve32r"), not(all(target_arch = "aarch64", target_endian = "little", not(miri)))))]
compile_error!("experimental hb_backend requires little-endian AArch64 without Miri");
#[cfg(all(any(hb_backend = "sve16", hb_backend = "sve32", hb_backend = "sve16r", hb_backend = "sve32r"), not(target_feature = "sve")))]
compile_error!("SVE hb_backend requires -C target-feature=+sve");
#[cfg(all(hb_private_scratch, not(any(hb_backend = "sve16", hb_backend = "sve32"))))]
compile_error!("hb_private_scratch requires the compact sve16 or sve32 backend");

cfg_if! {
    if #[cfg(all(target_arch = "aarch64", target_endian = "little", not(miri), hb_private_scratch, any(hb_backend = "sve16", hb_backend = "sve32")))] {
        mod sve_private;
        use sve_private as imp;
    } else if #[cfg(all(target_arch = "aarch64", target_endian = "little", not(miri), any(hb_backend = "sve16r", hb_backend = "sve32r")))] {
        mod sve_register;
        use sve_register as imp;
    } else if #[cfg(all(target_arch = "aarch64", target_endian = "little", not(miri), any(hb_backend = "sve16", hb_backend = "sve32")))] {
        mod sve;
        use sve as imp;
    } else if #[cfg(all(target_arch = "aarch64", target_endian = "little", not(miri), hb_backend = "neon16"))] {
        mod neon16;
        use neon16 as imp;
    } else
    // Use the SSE2 implementation if possible: it allows us to scan 16 buckets
    // at once instead of 8. We don't bother with AVX since it would require
    // runtime dispatch and wouldn't gain us much anyways: the probability of
    // finding a match drops off drastically after the first few buckets.
    //
    // I attempted an implementation on ARM using NEON instructions, but it
    // turns out that most NEON instructions have multi-cycle latency, which in
    // the end outweighs any gains over the generic implementation.
    if #[cfg(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri),
    ))] {
        mod sse2;
        use sse2 as imp;
    } else if #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        // NEON intrinsics are currently broken on big-endian targets.
        // See https://github.com/rust-lang/stdarch/issues/1484.
        target_endian = "little",
        not(miri),
    ))] {
        mod neon;
        use neon as imp;
    } else if #[cfg(all(
        feature = "nightly",
        target_arch = "loongarch64",
        target_feature = "lsx",
        not(miri),
    ))] {
        mod lsx;
        use lsx as imp;
    } else {
        mod generic;
        use generic as imp;
    }
}
pub(crate) use self::imp::Group;
pub(super) use self::imp::{BITMASK_ITER_MASK, BITMASK_STRIDE, BitMaskWord, NonZeroBitMaskWord};
