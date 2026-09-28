//! Experimental fixed-width SVE groups. SVE32 requires VL >= 32 bytes on
//! every executing thread, throughout the lifetime of the program. The
//! experiment launcher establishes that precondition before entering Rust.
//! Table layout and probe width never depend on a thread's current VL.

use super::super::{BitMask, Tag};
use core::arch::asm;
use core::{mem, ptr};

#[cfg(hb_backend = "sve16")]
pub(crate) type BitMaskWord = u16;
#[cfg(hb_backend = "sve16")]
pub(crate) type NonZeroBitMaskWord = core::num::NonZeroU16;
#[cfg(hb_backend = "sve32")]
pub(crate) type BitMaskWord = u32;
#[cfg(hb_backend = "sve32")]
pub(crate) type NonZeroBitMaskWord = core::num::NonZeroU32;
pub(crate) const BITMASK_STRIDE: usize = 1;
pub(crate) const BITMASK_ITER_MASK: BitMaskWord = !0;

#[cfg(hb_backend = "sve16")]
const WIDTH: usize = 16;
#[cfg(hb_backend = "sve32")]
const WIDTH: usize = 32;

// These macros expand to literals accepted by asm!, rather than depending
// on assembler-state directives or passing scalable values through vregs.
#[cfg(hb_backend = "sve16")]
macro_rules! active_lanes { () => { "ptrue p0.b, vl16" }; }
#[cfg(hb_backend = "sve32")]
macro_rules! active_lanes { () => { "ptrue p0.b, vl32" }; }
#[cfg(hb_backend = "sve16")]
macro_rules! read_mask { () => { "ldrh {mask:w}, [{scratch}]" }; }
#[cfg(hb_backend = "sve32")]
macro_rules! read_mask { () => { "ldr {mask:w}, [{scratch}]" }; }

/// An owned snapshot, including when retained by RawIterHashIndices.
#[derive(Copy, Clone)]
#[repr(C)]
#[cfg_attr(hb_backend = "sve16", repr(align(16)))]
#[cfg_attr(hb_backend = "sve32", repr(align(32)))]
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

    /// Read exactly WIDTH valid bytes and return the two lookup results in
    /// one asm block. No SVE register escapes across a Rust call or closure.
    #[inline]
    pub(crate) unsafe fn scan_lookup(ctrl: *const Tag, tag: Tag) -> (BitMask, bool) {
        Self::check_vl();
        // STR predicate writes the entire hardware predicate even for VL16
        // active lanes. 32 bytes cover the maximum architectural VL (2048b).
        // Only the low initialized bytes are read, inside this same asm.
        let mut scratch = mem::MaybeUninit::<[u64; 4]>::uninit();
        let mask: u32;
        let empty: u32;
        unsafe {
            asm!(
                active_lanes!(),
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                "dup z1.b, {tag:w}",
                "cmpeq p1.b, p0/z, z0.b, z1.b",
                "str p1, [{scratch}]",
                read_mask!(),
                "cmpeq p2.b, p0/z, z0.b, #-1",
                "cset {empty:w}, ne",
                ctrl = in(reg) ctrl,
                tag = in(reg) u32::from(tag.0),
                scratch = in(reg) scratch.as_mut_ptr(),
                mask = out(reg) mask,
                empty = out(reg) empty,
                out("v0") _, out("v1") _,
                out("p0") _, out("p1") _, out("p2") _,
                options(nostack),
            );
        }
        (BitMask(mask as BitMaskWord), empty != 0)
    }

    /// Fused insertion scan. The first-special sentinel is WIDTH; an earlier
    /// DELETED slot may be remembered while probing onward until EMPTY.
    #[cfg(hb_fused_insert)]
    #[inline]
    pub(crate) unsafe fn scan_insert(ctrl: *const Tag, tag: Tag) -> (BitMask, bool, usize) {
        Self::check_vl();
        let mut scratch = mem::MaybeUninit::<[u64; 4]>::uninit();
        let mask: u32;
        let empty: u32;
        let first: usize;
        unsafe {
            asm!(
                active_lanes!(),
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                "dup z1.b, {tag:w}",
                "mov {mask:w}, wzr",
                "cmpeq p1.b, p0/z, z0.b, z1.b",
                // A new unique key usually has no H2 candidates. Consume
                // CMPEQ's Z flag before another predicate comparison changes
                // it, avoiding the predicate store/load dependency entirely.
                "b.eq 2f",
                "str p1, [{scratch}]",
                read_mask!(),
                "2:",
                "cmpeq p2.b, p0/z, z0.b, #-1",
                "cset {empty:w}, ne",
                "cmplt p2.b, p0/z, z0.b, #0",
                "brkb p3.b, p0/z, p2.b",
                "cntp {first}, p0, p3.b",
                ctrl = in(reg) ctrl,
                tag = in(reg) u32::from(tag.0),
                scratch = in(reg) scratch.as_mut_ptr(),
                mask = out(reg) mask,
                empty = out(reg) empty,
                first = out(reg) first,
                out("v0") _, out("v1") _,
                out("p0") _, out("p1") _, out("p2") _, out("p3") _,
                options(nostack),
            );
        }
        (BitMask(mask as BitMaskWord), empty != 0, first)
    }

    /// Used when the key is already known to be absent, including rehash.
    /// BRKB followed by CNTP returns WIDTH for an entirely full group.
    #[cfg(hb_fused_insert)]
    #[inline]
    pub(crate) unsafe fn scan_first_special(ctrl: *const Tag) -> usize {
        Self::check_vl();
        let first: usize;
        unsafe {
            asm!(
                active_lanes!(),
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                "cmplt p1.b, p0/z, z0.b, #0",
                "brkb p2.b, p0/z, p1.b",
                "cntp {first}, p0, p2.b",
                ctrl = in(reg) ctrl,
                first = out(reg) first,
                out("v0") _, out("p0") _, out("p1") _, out("p2") _,
                options(pure, readonly, nostack),
            );
        }
        first
    }

    #[inline]
    pub(crate) fn match_tag(self, tag: Tag) -> BitMask {
        Self::check_vl();
        let mut scratch = mem::MaybeUninit::<[u64; 4]>::uninit();
        let mask: u32;
        unsafe {
            asm!(
                active_lanes!(),
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                "dup z1.b, {tag:w}",
                "cmpeq p1.b, p0/z, z0.b, z1.b",
                "str p1, [{scratch}]",
                read_mask!(),
                ctrl = in(reg) self.0.as_ptr(),
                tag = in(reg) u32::from(tag.0),
                scratch = in(reg) scratch.as_mut_ptr(),
                mask = out(reg) mask,
                out("v0") _, out("v1") _, out("p0") _, out("p1") _,
                options(nostack),
            );
        }
        BitMask(mask as BitMaskWord)
    }

    #[inline]
    pub(crate) fn match_empty(self) -> BitMask { self.match_tag(Tag::EMPTY) }

    #[inline]
    pub(crate) fn match_empty_or_deleted(self) -> BitMask {
        Self::check_vl();
        let mut scratch = mem::MaybeUninit::<[u64; 4]>::uninit();
        let mask: u32;
        unsafe {
            asm!(
                active_lanes!(),
                "ld1b {{z0.b}}, p0/z, [{ctrl}]",
                "cmplt p1.b, p0/z, z0.b, #0",
                "str p1, [{scratch}]",
                read_mask!(),
                ctrl = in(reg) self.0.as_ptr(),
                scratch = in(reg) scratch.as_mut_ptr(),
                mask = out(reg) mask,
                out("v0") _, out("p0") _, out("p1") _,
                options(nostack),
            );
        }
        BitMask(mask as BitMaskWord)
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
