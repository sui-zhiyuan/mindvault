mod bitmask;
mod group;
mod tag;

#[cfg(all(test, target_arch = "aarch64", any(hb_backend = "neon16", hb_backend = "sve16", hb_backend = "sve32", hb_backend = "sve16r", hb_backend = "sve32r")))]
mod tests;

use self::bitmask::BitMask;
pub(crate) use self::{
    bitmask::BitMaskIter,
    group::Group,
    tag::{Tag, TagSliceExt},
};
