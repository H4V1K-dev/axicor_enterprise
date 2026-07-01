use std::fs::remove_file;
use std::io::Write;
use std::path::PathBuf;
use vfs::{pack_entries, ArchiveEntry, AxicArchive, VfsError};

fn temp_archive_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("test_archive_{}.axic", rand));
    path
}

fn write_temp_archive(bytes: &[u8]) -> PathBuf {
    let path = temp_archive_path();
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(bytes).unwrap();
    path
}

#[test]
fn test_valid_archive_open() {
    let entries = vec![
        ArchiveEntry {
            path: "config/model.toml",
            bytes: b"hello model",
        },
        ArchiveEntry {
            path: "state.bin",
            bytes: b"some state bytes",
        },
    ];

    let packed = pack_entries(&entries).unwrap();
    let path = write_temp_archive(&packed);

    let archive = AxicArchive::open(&path).unwrap();

    assert!(archive.contains("config/model.toml"));
    assert!(archive.contains("state.bin"));
    assert!(!archive.contains("missing.txt"));

    assert_eq!(
        archive.get_file("config/model.toml"),
        Some(b"hello model".as_slice())
    );
    assert_eq!(
        archive.get_file("state.bin"),
        Some(b"some state bytes".as_slice())
    );
    assert_eq!(archive.get_file("missing.txt"), None);

    assert_eq!(
        archive.require_file("config/model.toml").unwrap(),
        b"hello model".as_slice()
    );
    assert!(matches!(
        archive.require_file("missing.txt"),
        Err(VfsError::FileNotFound)
    ));

    let mut list: Vec<&str> = archive.list_files().collect();
    list.sort();
    assert_eq!(list, vec!["config/model.toml", "state.bin"]);

    remove_file(path).unwrap();
}

#[test]
fn test_reject_bad_magic_and_version() {
    // Bad Magic
    let packed = pack_entries(&[]).unwrap();
    let mut bad_magic = packed.clone();
    bad_magic[0..4].copy_from_slice(b"BXIC");
    let path = write_temp_archive(&bad_magic);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::InvalidMagic)
    ));
    remove_file(path).unwrap();

    // Bad Version
    let mut bad_version = packed.clone();
    bad_version[4..8].copy_from_slice(&2u32.to_le_bytes());
    let path = write_temp_archive(&bad_version);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::UnsupportedVersion)
    ));
    remove_file(path).unwrap();
}

#[test]
fn test_reject_truncated_header() {
    let path = write_temp_archive(&[0u8; 10]);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::HeaderTooSmall)
    ));
    remove_file(path).unwrap();

    // Truncated TOC
    let mut header = Vec::new();
    header.extend_from_slice(b"AXIC");
    header.extend_from_slice(&1u32.to_le_bytes());
    header.extend_from_slice(&5u32.to_le_bytes()); // Expects 5 files (TOC size = 5 * 272 = 1360)
    let path = write_temp_archive(&header);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::TocOutOfBounds)
    ));
    remove_file(path).unwrap();
}

#[test]
fn test_reject_toc_out_of_bounds() {
    let entries = vec![ArchiveEntry {
        path: "a.txt",
        bytes: b"hello",
    }];
    let mut packed = pack_entries(&entries).unwrap();

    // Modify TOC entry offset of the first file (offset starts at byte 12 + 256 = 268)
    // Put aligned but out-of-bounds offset (e.g. 409600)
    let invalid_offset = 409600u64;
    packed[268..276].copy_from_slice(&invalid_offset.to_le_bytes());
    let path = write_temp_archive(&packed);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::EntryOutOfBounds)
    ));
    remove_file(path).unwrap();
}

#[test]
fn test_reject_integer_overflow() {
    let entries = vec![ArchiveEntry {
        path: "a.txt",
        bytes: b"hello",
    }];
    let mut packed = pack_entries(&entries).unwrap();

    // Put aligned offset = 4096 (byte 268) and size = u64::MAX (byte 276)
    let offset = 4096u64;
    packed[268..276].copy_from_slice(&offset.to_le_bytes());
    packed[276..284].copy_from_slice(&u64::MAX.to_le_bytes());
    let path = write_temp_archive(&packed);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::OffsetOverflow)
    ));
    remove_file(path).unwrap();
}

#[test]
fn test_reject_duplicate_normalized_paths() {
    let entries = vec![
        ArchiveEntry {
            path: "a.txt",
            bytes: b"1",
        },
        ArchiveEntry {
            path: "a.txt",
            bytes: b"2",
        },
    ];
    assert!(matches!(
        pack_entries(&entries),
        Err(VfsError::DuplicatePath)
    ));
}

#[test]
fn test_reject_path_traversal() {
    let bad_paths = vec!["../a", "a/../b", "./a", "a/.", "a/..", "a/./b"];
    for path in bad_paths {
        let entries = vec![ArchiveEntry { path, bytes: b"1" }];
        assert!(
            matches!(pack_entries(&entries), Err(VfsError::InvalidPath)),
            "Should reject traversal path: {}",
            path
        );
    }
}

#[test]
fn test_reject_non_terminated_path() {
    let entries = vec![ArchiveEntry {
        path: "a.txt",
        bytes: b"hello",
    }];
    let mut packed = pack_entries(&entries).unwrap();

    // Fill path field (12..268) with 'a' (no null byte)
    packed[12..268].fill(b'a');
    let path = write_temp_archive(&packed);
    assert!(matches!(
        AxicArchive::open(&path),
        Err(VfsError::PathNotTerminated)
    ));
    remove_file(path).unwrap();
}

#[test]
fn test_reject_invalid_archive_paths() {
    let invalid_paths = vec![
        "a\\b",   // backslash
        "C:file", // drive prefix
        "/a",     // leading slash
        "a//b",   // empty segment
        "a/",     // trailing slash
        "a\0b",   // nul inside
    ];
    for path in invalid_paths {
        let entries = vec![ArchiveEntry { path, bytes: b"1" }];
        assert!(
            matches!(pack_entries(&entries), Err(VfsError::InvalidPath)),
            "Should reject: {}",
            path
        );
    }
}

#[test]
fn test_zero_size_file_returns_empty_slice() {
    let entries = vec![ArchiveEntry {
        path: "empty.txt",
        bytes: &[],
    }];
    let packed = pack_entries(&entries).unwrap();
    let path = write_temp_archive(&packed);
    let archive = AxicArchive::open(&path).unwrap();

    assert!(archive.contains("empty.txt"));
    assert_eq!(archive.get_file("empty.txt"), Some(&[][..]));
    assert_eq!(archive.require_file("empty.txt").unwrap(), &[][..]);

    remove_file(path).unwrap();
}

#[test]
fn test_empty_archive_handling() {
    let packed = pack_entries(&[]).unwrap();
    assert_eq!(packed.len(), 12); // strictly 12 bytes: magic (4) + version (4) + count (4)

    let path = write_temp_archive(&packed);
    let archive = AxicArchive::open(&path).unwrap();

    assert_eq!(archive.list_files().count(), 0);
    assert_eq!(archive.get_file("any.txt"), None);
    assert!(matches!(
        archive.require_file("any.txt"),
        Err(VfsError::FileNotFound)
    ));

    remove_file(path).unwrap();
}

#[test]
fn test_packer_deterministic_output() {
    let entries1 = vec![
        ArchiveEntry {
            path: "b.txt",
            bytes: b"bb",
        },
        ArchiveEntry {
            path: "a.txt",
            bytes: b"a",
        },
    ];
    let entries2 = vec![
        ArchiveEntry {
            path: "a.txt",
            bytes: b"a",
        },
        ArchiveEntry {
            path: "b.txt",
            bytes: b"bb",
        },
    ];

    let packed1 = pack_entries(&entries1).unwrap();
    let packed2 = pack_entries(&entries2).unwrap();

    assert_eq!(packed1, packed2);
}

#[test]
fn test_payload_offset_alignment() {
    let entries = vec![
        ArchiveEntry {
            path: "a.txt",
            bytes: b"a",
        },
        ArchiveEntry {
            path: "b.txt",
            bytes: &[0u8; 5000],
        },
        ArchiveEntry {
            path: "c.txt",
            bytes: b"c",
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = write_temp_archive(&packed);
    let archive = AxicArchive::open(&path).unwrap();

    // Check sorted list
    let list: Vec<&str> = archive.list_files().collect();
    assert_eq!(list, vec!["a.txt", "b.txt", "c.txt"]);

    // Manually extract and check alignments of offsets in the binary data
    for i in 0..3 {
        let entry_offset = 12 + i * 272;
        let file_offset = u64::from_le_bytes(
            packed[entry_offset + 256..entry_offset + 264]
                .try_into()
                .unwrap(),
        );
        assert_eq!(
            file_offset % 4096,
            0,
            "Offset of entry {} must be aligned to 4096",
            i
        );
    }

    remove_file(path).unwrap();
}

#[test]
fn test_archive_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AxicArchive>();
}

#[test]
fn test_invalid_lookup_paths() {
    let entries = vec![ArchiveEntry {
        path: "a.txt",
        bytes: b"hello",
    }];
    let packed = pack_entries(&entries).unwrap();
    let path = write_temp_archive(&packed);
    let archive = AxicArchive::open(&path).unwrap();

    // get_file with invalid path should return None, not error/panic
    assert_eq!(archive.get_file("../a.txt"), None);
    assert_eq!(archive.get_file("a\\txt"), None);

    // require_file with invalid path should return VfsError::InvalidPath
    assert!(matches!(
        archive.require_file("../a.txt"),
        Err(VfsError::InvalidPath)
    ));

    remove_file(path).unwrap();
}

#[test]
fn test_list_files_order() {
    let entries = vec![
        ArchiveEntry {
            path: "z.txt",
            bytes: b"z",
        },
        ArchiveEntry {
            path: "m.txt",
            bytes: b"m",
        },
        ArchiveEntry {
            path: "a.txt",
            bytes: b"a",
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = write_temp_archive(&packed);
    let archive = AxicArchive::open(&path).unwrap();

    let list: Vec<&str> = archive.list_files().collect();
    // Must be lexicographically sorted: a.txt, m.txt, z.txt
    assert_eq!(list, vec!["a.txt", "m.txt", "z.txt"]);

    remove_file(path).unwrap();
}
