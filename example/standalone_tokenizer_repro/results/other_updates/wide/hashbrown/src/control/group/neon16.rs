use super::super::{BitMask, Tag};
use core::arch::aarch64 as neon;
use core::mem;
use core::num::NonZeroU64;

pub(crate) type BitMaskWord = u64;
pub(crate) type NonZeroBitMaskWord = NonZeroU64;
pub(crate) const BITMASK_STRIDE: usize = 4;
pub(crate) const BITMASK_ITER_MASK: BitMaskWord = 0x8888_8888_8888_8888;

/// Sixteen control bytes, with four mask bits per lane.
#[derive(Copy, Clone)]
pub(crate) struct Group(neon::uint8x16_t);

impl Group {
    pub(crate) const WIDTH: usize = 16;

    #[inline]
    pub(crate) const fn static_empty() -> &'static [Tag; Self::WIDTH] {
        #[repr(align(16))]
        struct Aligned([Tag; Group::WIDTH]);
        const EMPTY: Aligned = Aligned([Tag::EMPTY; Group::WIDTH]);
        &EMPTY.0
    }

    #[inline]
    pub(crate) unsafe fn load(ptr: *const Tag) -> Self {
        unsafe { Self(neon::vld1q_u8(ptr.cast())) }
    }

    #[inline]
    pub(crate) unsafe fn load_aligned(ptr: *const Tag) -> Self {
        debug_assert_eq!(ptr.align_offset(mem::align_of::<Self>()), 0);
        unsafe { Self::load(ptr) }
    }

    #[inline]
    pub(crate) unsafe fn store_aligned(self, ptr: *mut Tag) {
        debug_assert_eq!(ptr.align_offset(mem::align_of::<Self>()), 0);
        unsafe { neon::vst1q_u8(ptr.cast(), self.0) }
    }

    // Each compare byte is 00 or ff. SHRN packs adjacent bytes into two
    // four-bit fields, preserving address order on little-endian AArch64.
    #[inline]
    unsafe fn mask(cmp: neon::uint8x16_t) -> BitMask {
        unsafe {
            let packed = neon::vshrn_n_u16::<4>(neon::vreinterpretq_u16_u8(cmp));
            BitMask(neon::vget_lane_u64(neon::vreinterpret_u64_u8(packed), 0))
        }
    }

    #[inline]
    pub(crate) fn match_tag(self, tag: Tag) -> BitMask {
        unsafe { Self::mask(neon::vceqq_u8(self.0, neon::vdupq_n_u8(tag.0))) }
    }

    #[inline]
    pub(crate) fn match_empty(self) -> BitMask { self.match_tag(Tag::EMPTY) }

    #[inline]
    pub(crate) fn match_empty_or_deleted(self) -> BitMask {
        unsafe { Self::mask(neon::vcltzq_s8(neon::vreinterpretq_s8_u8(self.0))) }
    }

    #[inline]
    pub(crate) fn match_full(self) -> BitMask {
        unsafe { Self::mask(neon::vcgezq_s8(neon::vreinterpretq_s8_u8(self.0))) }
    }

    #[inline]
    pub(crate) fn convert_special_to_empty_and_full_to_deleted(self) -> Self {
        unsafe {
            let special = neon::vcltzq_s8(neon::vreinterpretq_s8_u8(self.0));
            Self(neon::vorrq_u8(special, neon::vdupq_n_u8(0x80)))
        }
    }
}
