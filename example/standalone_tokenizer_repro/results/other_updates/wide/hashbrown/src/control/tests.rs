use super::{Group, Tag};

#[cfg(all(hb_private_scratch, any(hb_backend = "sve16", hb_backend = "sve32")))]
#[test]
fn private_scratch_backend_selected() {
    assert!(core::any::type_name::<Group>().contains("sve_private"));
}

#[cfg(all(hb_fused_insert, any(hb_backend = "sve16", hb_backend = "sve32")))]
#[test]
fn fused_insert_scan_semantics() {
    // Full groups cover the WIDTH sentinel and both zero/nonzero match paths.
    // Each lane is then the first DELETED/EMPTY, including the last lane.
    for special_lane in 0..=Group::WIDTH {
        for special in [Tag::DELETED, Tag::EMPTY] {
            let mut tags = [Tag(42); Group::WIDTH];
            if special_lane < Group::WIDTH { tags[special_lane] = special; }
            for target in [Tag(42), Tag(17)] {
                let (matched, empty, first) = unsafe { Group::scan_insert(tags.as_ptr(), target) };
                let expected: std::vec::Vec<_> = tags.iter().enumerate()
                    .filter_map(|(lane, tag)| (*tag == target).then_some(lane)).collect();
                assert_eq!(matched.into_iter().collect::<std::vec::Vec<_>>(), expected);
                assert_eq!(empty, special_lane < Group::WIDTH && special == Tag::EMPTY);
                assert_eq!(first, special_lane);
                assert_eq!(unsafe { Group::scan_first_special(tags.as_ptr()) }, special_lane);
            }
        }
    }
    let mut tags = [Tag(17); Group::WIDTH];
    tags[1] = Tag::DELETED;
    tags[Group::WIDTH - 1] = Tag::EMPTY;
    let (_, empty, first) = unsafe { Group::scan_insert(tags.as_ptr(), Tag(42)) };
    assert!(empty);
    assert_eq!(first, 1);
}

#[test]
fn selected_backend_width() {
    let expected = if cfg!(any(hb_backend = "sve32", hb_backend = "sve32r")) { 32 } else { 16 };
    assert_eq!(Group::WIDTH, expected);
    assert_eq!(Group::static_empty().as_ptr() as usize % expected, 0);
}

#[test]
fn loaded_group_is_a_snapshot() {
    let mut tags = [Tag(42); Group::WIDTH];
    let snapshot = unsafe { Group::load(tags.as_ptr()) };
    tags.fill(Tag::EMPTY);
    assert!(tags.iter().all(|tag| *tag == Tag::EMPTY));
    assert_eq!(snapshot.match_tag(Tag(42)).into_iter().count(), Group::WIDTH);
    assert!(!snapshot.match_empty().any_bit_set());
}

#[test]
fn exact_masks_and_zero_counts() {
    let mut tags = [Tag::EMPTY; Group::WIDTH];
    for lane in 0..Group::WIDTH {
        tags.fill(Tag::EMPTY);
        tags[lane] = Tag(42);
        let group = unsafe { Group::load(tags.as_ptr()) };
        let matched = group.match_tag(Tag(42));
        assert_eq!(matched.into_iter().collect::<std::vec::Vec<_>>(), [lane]);
        assert_eq!(matched.trailing_zeros(), lane);
        assert_eq!(matched.leading_zeros(), Group::WIDTH - lane - 1);
        assert_eq!(group.match_full().into_iter().collect::<std::vec::Vec<_>>(), [lane]);
    }
    tags.fill(Tag(1));
    let group = unsafe { Group::load(tags.as_ptr()) };
    let empty = group.match_empty();
    assert!(!empty.any_bit_set());
    assert_eq!(empty.leading_zeros(), Group::WIDTH);
    assert_eq!(empty.trailing_zeros(), Group::WIDTH);
    assert_eq!(group.match_full().into_iter().count(), Group::WIDTH);
}

#[test]
fn all_tags_unaligned_and_conversion() {
    #[repr(align(32))]
    struct Aligned([Tag; 96]);
    let mut storage = Aligned([Tag::EMPTY; 96]);
    for offset in 0..32 {
        for seed in 0..256usize {
            for lane in 0..Group::WIDTH {
                storage.0[offset + lane] = match (seed + lane) % 5 {
                    0 => Tag::EMPTY,
                    1 => Tag::DELETED,
                    _ => Tag(((seed + lane * 17) % 128) as u8),
                };
            }
            let tags = &storage.0[offset..offset + Group::WIDTH];
            let group = unsafe { Group::load(tags.as_ptr()) };
            #[cfg(all(hb_fused_insert, any(hb_backend = "sve16", hb_backend = "sve32")))]
            {
                let target = Tag((seed % 128) as u8);
                let (matched, empty, first) = unsafe { Group::scan_insert(tags.as_ptr(), target) };
                let expected: std::vec::Vec<_> = tags.iter().enumerate()
                    .filter_map(|(lane, tag)| (*tag == target).then_some(lane)).collect();
                let first_special = tags.iter().position(|tag| tag.is_special()).unwrap_or(Group::WIDTH);
                assert_eq!(matched.into_iter().collect::<std::vec::Vec<_>>(), expected);
                assert_eq!(empty, tags.contains(&Tag::EMPTY));
                assert_eq!(first, first_special);
                assert_eq!(unsafe { Group::scan_first_special(tags.as_ptr()) }, first_special);
            }
            for target in 0..128 {
                let expected: std::vec::Vec<_> = tags.iter().enumerate()
                    .filter_map(|(i, t)| (t.0 == target).then_some(i)).collect();
                assert_eq!(group.match_tag(Tag(target)).into_iter().collect::<std::vec::Vec<_>>(), expected);
                #[cfg(any(hb_backend = "sve16", hb_backend = "sve32", hb_backend = "sve16r", hb_backend = "sve32r"))]
                {
                    let (matched, has_empty) = unsafe { Group::scan_lookup(tags.as_ptr(), Tag(target)) };
                    assert_eq!(matched.into_iter().collect::<std::vec::Vec<_>>(), expected);
                    assert_eq!(has_empty, tags.iter().any(|t| t.0 == 255));
                }
            }
            let empty: std::vec::Vec<_> = tags.iter().enumerate()
                .filter_map(|(i, t)| (t.0 == 255).then_some(i)).collect();
            let special: std::vec::Vec<_> = tags.iter().enumerate()
                .filter_map(|(i, t)| (t.0 >= 128).then_some(i)).collect();
            assert_eq!(group.match_empty().into_iter().collect::<std::vec::Vec<_>>(), empty);
            assert_eq!(group.match_empty_or_deleted().into_iter().collect::<std::vec::Vec<_>>(), special);
            let converted = group.convert_special_to_empty_and_full_to_deleted();
            let mut output = Aligned([Tag(0); 96]);
            unsafe { converted.store_aligned(output.0.as_mut_ptr()) };
            for (input, actual) in tags.iter().zip(output.0.iter()) {
                assert_eq!(actual.0, if input.0 >= 128 { 255 } else { 128 });
            }
        }
    }
}

// A load ending exactly at an inaccessible page catches accidental full-VL
// loads in SVE16 as well as over-reads in the other backends.
#[cfg(target_os = "linux")]
#[test]
fn load_at_guard_page() {
    unsafe extern "C" {
        fn getpagesize() -> core::ffi::c_int;
        fn mmap(addr: *mut core::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, off: isize) -> *mut core::ffi::c_void;
        fn mprotect(addr: *mut core::ffi::c_void, len: usize, prot: i32) -> i32;
        fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    }
    unsafe {
        let page = getpagesize() as usize;
        let allocation = mmap(core::ptr::null_mut(), page * 2, 3, 0x22, -1, 0);
        assert_ne!(allocation as isize, -1);
        assert_eq!(mprotect(allocation.cast::<u8>().add(page).cast(), page, 0), 0);
        let tags = allocation.cast::<Tag>().add(page - Group::WIDTH);
        for lane in 0..Group::WIDTH { tags.add(lane).write(Tag(42)); }
        assert_eq!(Group::load(tags).match_tag(Tag(42)).into_iter().count(), Group::WIDTH);
        #[cfg(all(hb_fused_insert, any(hb_backend = "sve16", hb_backend = "sve32")))]
        {
            let (matched, empty, first) = Group::scan_insert(tags, Tag(42));
            assert_eq!(matched.into_iter().count(), Group::WIDTH);
            assert!(!empty);
            assert_eq!(first, Group::WIDTH);
            assert_eq!(Group::scan_first_special(tags), Group::WIDTH);
        }
        #[cfg(any(hb_backend = "sve16", hb_backend = "sve32", hb_backend = "sve16r", hb_backend = "sve32r"))]
        {
            let (matched, has_empty) = Group::scan_lookup(tags, Tag(42));
            assert_eq!(matched.into_iter().count(), Group::WIDTH);
            assert!(!has_empty);
        }
        assert_eq!(munmap(allocation, page * 2), 0);
    }
}
