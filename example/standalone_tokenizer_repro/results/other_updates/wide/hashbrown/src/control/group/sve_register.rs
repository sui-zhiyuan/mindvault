//! Register-only mask extraction for attribution against the compact SVE
//! backend. Logical widths are fixed; SVE32r requires VL >= 32 bytes on all
//! executing threads, as established by the experiment launcher.

use super::super::{BitMask, Tag};
use core::arch::asm;
use core::{mem, ptr};

#[cfg(hb_backend = "sve16r")]
pub(crate) type BitMaskWord = u64;
#[cfg(hb_backend = "sve16r")]
pub(crate) type NonZeroBitMaskWord = core::num::NonZeroU64;
#[cfg(hb_backend = "sve32r")]
pub(crate) type BitMaskWord = u128;
#[cfg(hb_backend = "sve32r")]
pub(crate) type NonZeroBitMaskWord = core::num::NonZeroU128;
pub(crate) const BITMASK_STRIDE: usize = 4;
// One bit per nibble, repeated across the entire mask word.
pub(crate) const BITMASK_ITER_MASK: BitMaskWord = (!0 / 15) * 8;

#[cfg(hb_backend = "sve16r")]
const WIDTH: usize = 16;
#[cfg(hb_backend = "sve32r")]
const WIDTH: usize = 32;

// The comparison writes p1. Extraction materializes each predicate bit as
// ff/00, then SHRN packs adjacent bytes into four-bit fields. Crucially the
// first SHRN writes v2, not v1: a NEON write to v1 would zero z1's upper half
// before SVE32r extracts it. Inputs, extraction, and optional empty test all
// remain in one block; no scalable value crosses a Rust asm boundary.
macro_rules! register_mask {
    ($ctrl:expr, [$($compare:literal),+], [$($tail:literal),*], $($operands:tt)*) => {{
        let low: u64;
        #[cfg(hb_backend = "sve32r")]
        let high: u64;
        #[cfg(hb_backend = "sve16r")]
        unsafe {
            asm!(
                "ptrue p0.b, vl16",
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                $($compare,)+
                "mov z1.b, p1/z, #-1",
                "shrn v2.8b, v1.8h, #4",
                "fmov {low}, d2",
                $($tail,)*
                ctrl = in(reg) $ctrl,
                low = out(reg) low,
                $($operands)*
                out("v0") _, out("v1") _, out("v2") _,
                out("p0") _, out("p1") _, out("p2") _,
                options(pure, readonly, nostack),
            );
        }
        #[cfg(hb_backend = "sve32r")]
        unsafe {
            asm!(
                "ptrue p0.b, vl32",
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                $($compare,)+
                "mov z1.b, p1/z, #-1",
                "shrn v2.8b, v1.8h, #4",
                "fmov {low}, d2",
                "ext z1.b, z1.b, z1.b, #16",
                "shrn v2.8b, v1.8h, #4",
                "fmov {high}, d2",
                $($tail,)*
                ctrl = in(reg) $ctrl,
                low = out(reg) low,
                high = out(reg) high,
                $($operands)*
                out("v0") _, out("v1") _, out("v2") _,
                out("p0") _, out("p1") _, out("p2") _,
                options(pure, readonly, nostack),
            );
        }
        #[cfg(hb_backend = "sve16r")]
        { BitMask(low) }
        #[cfg(hb_backend = "sve32r")]
        { BitMask(u128::from(low) | (u128::from(high) << 64)) }
    }};
}

#[derive(Copy, Clone)]
#[repr(C)]
#[cfg_attr(hb_backend = "sve16r", repr(align(16)))]
#[cfg_attr(hb_backend = "sve32r", repr(align(32)))]
pub(crate) struct Group([Tag; WIDTH]);

impl Group {
    pub(crate) const WIDTH: usize = WIDTH;

    #[inline]
    pub(crate) const fn static_empty() -> &'static [Tag; Self::WIDTH] {
        const EMPTY: Group = Group([Tag::EMPTY; WIDTH]);
        &EMPTY.0
    }

    #[inline]
    fn check_vl() {
        #[cfg(debug_assertions)]
        unsafe {
            let bytes: usize;
            asm!("cntb {bytes}", bytes = out(reg) bytes, options(nomem, nostack, preserves_flags));
            assert!(bytes >= WIDTH, "SVE backend requires VL >= Group::WIDTH on every thread");
        }
    }

    #[inline]
    pub(crate) unsafe fn load(ptr: *const Tag) -> Self {
        unsafe { ptr::read_unaligned(ptr.cast()) }
    }

    #[inline]
    pub(crate) unsafe fn load_aligned(ptr: *const Tag) -> Self {
        debug_assert_eq!(ptr.align_offset(mem::align_of::<Self>()), 0);
        unsafe { ptr::read(ptr.cast()) }
    }

    #[inline]
    pub(crate) unsafe fn store_aligned(self, ptr: *mut Tag) {
        debug_assert_eq!(ptr.align_offset(mem::align_of::<Self>()), 0);
        unsafe { ptr::write(ptr.cast(), self) }
    }

    #[inline]
    pub(crate) unsafe fn scan_lookup(ctrl: *const Tag, tag: Tag) -> (BitMask, bool) {
        Self::check_vl();
        let empty: u32;
        let mask = register_mask!(ctrl,
            ["dup z1.b, {tag:w}", "cmpeq p1.b, p0/z, z0.b, z1.b"],
            ["cmpeq p2.b, p0/z, z0.b, #-1", "cset {empty:w}, ne"],
            tag = in(reg) u32::from(tag.0),
            empty = out(reg) empty,
        );
        (mask, empty != 0)
    }

    #[inline]
    pub(crate) fn match_tag(self, tag: Tag) -> BitMask {
        Self::check_vl();
        register_mask!(self.0.as_ptr(),
            ["dup z1.b, {tag:w}", "cmpeq p1.b, p0/z, z0.b, z1.b"], [],
            tag = in(reg) u32::from(tag.0),
        )
    }

    #[inline]
    pub(crate) fn match_empty(self) -> BitMask { self.match_tag(Tag::EMPTY) }

    #[inline]
    pub(crate) fn match_empty_or_deleted(self) -> BitMask {
        Self::check_vl();
        register_mask!(self.0.as_ptr(), ["cmplt p1.b, p0/z, z0.b, #0"], [],)
    }

    #[inline]
    pub(crate) fn match_full(self) -> BitMask {
        BitMask(!self.match_empty_or_deleted().0)
    }

    #[inline]
    pub(crate) fn convert_special_to_empty_and_full_to_deleted(mut self) -> Self {
        for tag in &mut self.0 {
            *tag = if tag.is_special() { Tag::EMPTY } else { Tag::DELETED };
        }
        self
    }
}
