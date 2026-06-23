//! Core archive packing and loading algorithms.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use memmap2::Mmap;
use crate::error::VfsError;

/// Object representing a memory-mapped `.axic` archive.
/// Allows Zero-Copy random access to files packed within it.
pub struct AxicArchive {
    mmap: Mmap,
    /// TOC mapping paths to their respective (offset, size) values in bytes.
    pub toc: HashMap<String, (usize, usize)>,
}

impl std::fmt::Debug for AxicArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxicArchive")
            .field("toc", &self.toc)
            .finish()
    }
}

impl AxicArchive {
    /// Opens the `.axic` archive file, memory maps it, and parses the Table of Contents (TOC).
    ///
    /// # Arguments
    /// * `path` - Path to the archive file on disk.
    ///
    /// # Errors
    /// Returns `VfsError` if memory mapping fails, the archive header is invalid,
    /// or structural invariants (alignment, page limits) are violated.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, VfsError> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let file_len = metadata.len() as usize;

        // Truncated check (E-038)
        if file_len < crate::constants::AXIC_HEADER_SIZE {
            return Err(VfsError::OutOfBounds {
                offset: 0,
                size: crate::constants::AXIC_HEADER_SIZE,
                archive_size: file_len,
            });
        }

        // Memory map the file strictly Read-Only (INV-VFS-003)
        let mmap = unsafe { Mmap::map(&file).map_err(VfsError::MmapFailed)? };

        // Parse header bytes
        let magic = [mmap[0], mmap[1], mmap[2], mmap[3]];
        if magic != crate::constants::AXIC_MAGIC.to_le_bytes() {
            return Err(VfsError::InvalidMagic {
                expected: crate::constants::AXIC_MAGIC.to_le_bytes(),
                actual: magic,
            });
        }

        let version = u32::from_le_bytes([mmap[4], mmap[5], mmap[6], mmap[7]]);
        if version != 1 {
            return Err(VfsError::InvalidVersion(version));
        }

        let file_count = u32::from_le_bytes([mmap[8], mmap[9], mmap[10], mmap[11]]) as usize;

        // Parse TOC (Table of Contents)
        let mut toc = HashMap::with_capacity(file_count);
        let mut current_offset = crate::constants::AXIC_HEADER_SIZE;

        for _ in 0..file_count {
            if current_offset + crate::constants::TOC_ENTRY_SIZE > file_len {
                return Err(VfsError::OutOfBounds {
                    offset: current_offset,
                    size: crate::constants::TOC_ENTRY_SIZE,
                    archive_size: file_len,
                });
            }

            let entry_bytes = &mmap[current_offset..current_offset + crate::constants::TOC_ENTRY_SIZE];

            // Decode path string, searching for the first null byte '\0'
            let path_raw = &entry_bytes[0..256];
            let path_len = path_raw.iter().position(|&b| b == 0).unwrap_or(256);
            let path_utf8_slice = &path_raw[0..path_len];
            let path_str = std::str::from_utf8(path_utf8_slice)?;

            let file_offset = u64::from_le_bytes([
                entry_bytes[256], entry_bytes[257], entry_bytes[258], entry_bytes[259],
                entry_bytes[260], entry_bytes[261], entry_bytes[262], entry_bytes[263],
            ]) as usize;

            let file_size = u64::from_le_bytes([
                entry_bytes[264], entry_bytes[265], entry_bytes[266], entry_bytes[267],
                entry_bytes[268], entry_bytes[269], entry_bytes[270], entry_bytes[271],
            ]) as usize;

            // Enforce OS page alignment constraint (INV-VFS-001 / E-044)
            if file_offset % crate::constants::OS_PAGE_SIZE != 0 {
                return Err(VfsError::AlignmentViolation {
                    path: path_str.to_string(),
                    offset: file_offset,
                });
            }

            // Enforce memory bounds limits (E-040)
            if file_offset + file_size > file_len {
                return Err(VfsError::OutOfBounds {
                    offset: file_offset,
                    size: file_size,
                    archive_size: file_len,
                });
            }

            // Reject duplicates (INV-VFS-005)
            if toc.contains_key(path_str) {
                return Err(VfsError::DuplicatePath(path_str.to_string()));
            }

            toc.insert(path_str.to_string(), (file_offset, file_size));
            current_offset += crate::constants::TOC_ENTRY_SIZE;
        }

        // Check for overlap violation
        let mut entries: Vec<(&String, usize, usize)> = toc.iter()
            .map(|(path, &(offset, size))| (path, offset, size))
            .filter(|&(_, _, size)| size > 0)
            .collect();

        entries.sort_by_key(|&(_, offset, _)| offset);

        for i in 0..entries.len().saturating_sub(1) {
            let (path_a, offset_a, size_a) = entries[i];
            let (path_b, offset_b, size_b) = entries[i + 1];

            if offset_a + size_a > offset_b {
                return Err(VfsError::OverlapViolation {
                    path_a: (*path_a).clone(),
                    path_b: (*path_b).clone(),
                    offset_a,
                    size_a,
                    offset_b,
                    size_b,
                });
            }
        }

        Ok(Self { mmap, toc })
    }

    /// Retrieves a Zero-Copy slice referencing the mapped contents of a file.
    ///
    /// # Arguments
    /// * `path` - The logical path key of the requested file.
    pub fn get_file(&self, path: &str) -> Result<&[u8], VfsError> {
        if let Some(&(offset, size)) = self.toc.get(path) {
            if size == 0 {
                Ok(&[])
            } else {
                Ok(&self.mmap[offset..offset + size])
            }
        } else {
            Err(VfsError::FileNotFound(path.to_string()))
        }
    }

    /// Extracts a file from the archive and writes its contents to the destination path.
    ///
    /// Implements INV-CROSS-009 TMPFS Extraction.
    ///
    /// # Arguments
    /// * `path` - The logical path key of the file inside the archive.
    /// * `dest` - The path to save the extracted file to.
    ///
    /// # Errors
    /// Returns `VfsError` if retrieval fails, or if writing to `dest` fails.
    pub fn extract_file(&self, path: &str, dest: &Path) -> Result<(), VfsError> {
        let data = self.get_file(path)?;
        std::fs::write(dest, data).map_err(VfsError::IoError)?;
        Ok(())
    }
}

/// Calculates the size of padding bytes needed to align the current offset
/// to the OS page boundary (4096 bytes).
///
/// Implements [INV-VFS-001 OS Page Alignment](file:///w:/Workspace/Axicor/Docs/specs/spec_L2/vfs_spec.md#L52).
///
/// # Arguments
/// * `offset` - The current write position in bytes.
///
/// # Returns
/// The size of the padding in bytes (from 0 to 4095).
pub const fn page_padding(offset: usize) -> usize {
    (4096 - (offset % 4096)) % 4096
}

fn collect_files_recursive(
    dir: &Path,
    base: &Path,
    files: &mut Vec<(String, std::path::PathBuf, std::fs::Metadata)>,
) -> Result<(), VfsError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_files_recursive(&path, base, files)?;
        } else if file_type.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|_| VfsError::PathTooLong(path.to_string_lossy().into_owned()))?;
            let rel_str = rel.to_string_lossy().to_string().replace("\\", "/");
            if rel_str.len() > 255 {
                return Err(VfsError::PathTooLong(rel_str));
            }
            let metadata = entry.metadata()?;
            files.push((rel_str, path, metadata));
        }
    }
    Ok(())
}

/// Packs the contents of a directory recursively into a page-aligned archive.
///
/// Implements INV-VFS-001 OS Page Alignment.
///
/// # Arguments
/// * `project_dir` - The path of the directory to pack.
/// * `out_file` - The destination path of the output `.axic` archive file.
///
/// # Errors
/// Returns `VfsError` if reading/writing fails, if `project_dir` is not a directory,
/// or if a relative file path exceeds 255 bytes.
pub fn pack_directory(project_dir: &Path, out_file: &Path) -> Result<(), VfsError> {
    use std::io::{BufWriter, Write};

    if !project_dir.is_dir() {
        return Err(VfsError::NotADirectory(project_dir.to_path_buf()));
    }

    let mut entries = Vec::new();
    collect_files_recursive(project_dir, project_dir, &mut entries)?;

    // Sort entries to make packing deterministic
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let file_count = entries.len();
    let header_size = crate::constants::AXIC_HEADER_SIZE + file_count * crate::constants::TOC_ENTRY_SIZE;
    let mut current_offset = header_size + page_padding(header_size);

    let mut writer = BufWriter::new(File::create(out_file)?);

    // Write header: magic (4 bytes), version (4 bytes), file_count (4 bytes)
    writer.write_all(&crate::constants::AXIC_MAGIC.to_le_bytes())?;
    writer.write_all(&1u32.to_le_bytes())?;
    writer.write_all(&(file_count as u32).to_le_bytes())?;

    // Write TOC entries
    for (name_str, _path, metadata) in &entries {
        let file_size = metadata.len() as usize;
        let file_offset = current_offset;

        let mut path_bytes = [0u8; 256];
        let name_bytes = name_str.as_bytes();
        path_bytes[..name_bytes.len()].copy_from_slice(name_bytes);
        writer.write_all(&path_bytes)?;

        writer.write_all(&(file_offset as u64).to_le_bytes())?;
        writer.write_all(&(file_size as u64).to_le_bytes())?;

        let padding_after = page_padding(current_offset + file_size);
        current_offset += file_size + padding_after;
    }

    // Write initial padding zeros up to the start of the first file
    let initial_padding = page_padding(header_size);
    if initial_padding > 0 {
        let zeros = [0u8; 4096];
        writer.write_all(&zeros[..initial_padding])?;
    }

    // Write files and their respective padding
    let mut current_offset = header_size + page_padding(header_size);
    for (_name_str, path, metadata) in &entries {
        let file_size = metadata.len() as usize;

        let mut file = File::open(path)?;
        std::io::copy(&mut file, &mut writer)?;

        let current_padding = page_padding(current_offset + file_size);
        if current_padding > 0 {
            let zeros = [0u8; 4096];
            writer.write_all(&zeros[..current_padding])?;
        }
        current_offset += file_size + current_padding;
    }

    writer.flush()?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn write_temp_archive(data: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(data).unwrap();
        file.flush().unwrap();
        file
    }

    fn build_test_archive_raw(
        magic: [u8; 4],
        version: u32,
        files: &[(&[u8], u64, u64)],
        extra_data: &[u8],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&magic);
        data.extend_from_slice(&version.to_le_bytes());
        data.extend_from_slice(&(files.len() as u32).to_le_bytes());

        for &(path, offset, size) in files {
            let mut path_bytes = [0u8; 256];
            let len = path.len().min(255);
            path_bytes[..len].copy_from_slice(&path[..len]);
            data.extend_from_slice(&path_bytes);
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&size.to_le_bytes());
        }

        data.extend_from_slice(extra_data);
        data
    }

    #[test]
    fn test_padding_calculation() {
        assert_eq!(page_padding(0), 0);
        assert_eq!(page_padding(1), 4095);
        assert_eq!(page_padding(4095), 1);
        assert_eq!(page_padding(4096), 0);
        assert_eq!(page_padding(12), 4084);
    }

    #[test]
    fn test_truncated_archive_handling() {
        let temp = write_temp_archive(&[0, 1, 2]);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::OutOfBounds { .. } => {}
            other => panic!("Expected OutOfBounds error, got: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_magic_handling() {
        let raw = build_test_archive_raw(*b"FAIL", 1, &[], &[]);
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::InvalidMagic { expected, actual } => {
                assert_eq!(expected, crate::constants::AXIC_MAGIC.to_le_bytes());
                assert_eq!(actual, *b"FAIL");
            }
            other => panic!("Expected InvalidMagic error, got: {:?}", other),
        }
    }

    #[test]
    fn test_archive_version_mismatch() {
        let raw = build_test_archive_raw(crate::constants::AXIC_MAGIC.to_le_bytes(), 2, &[], &[]);
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::InvalidVersion(2) => {}
            other => panic!("Expected InvalidVersion(2) error, got: {:?}", other),
        }
    }

    #[test]
    fn test_unaligned_toc_error() {
        // file starts at offset 5000 (not page-aligned to 4096)
        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(b"test.txt", 5000, 100)],
            &vec![0u8; 6000],
        );
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::AlignmentViolation { path, offset } => {
                assert_eq!(path, "test.txt");
                assert_eq!(offset, 5000);
            }
            other => panic!("Expected AlignmentViolation error, got: {:?}", other),
        }
    }

    #[test]
    fn test_toc_bounds_check() {
        // file points to offset 8192 with size 4096, but archive bytes are smaller
        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(b"test.txt", 4096, 4096)],
            &[],
        );
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::OutOfBounds { .. } => {}
            other => panic!("Expected OutOfBounds error, got: {:?}", other),
        }
    }

    #[test]
    fn test_utf8_error_in_path() {
        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(&[0xFF, 0xFE, 0xFD], 4096, 10)],
            &vec![0u8; 5000],
        );
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::Utf8Error(_) => {}
            other => panic!("Expected Utf8Error error, got: {:?}", other),
        }
    }

    #[test]
    fn test_duplicate_path_handling() {
        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(b"dup.txt", 4096, 10), (b"dup.txt", 4096, 10)],
            &vec![0u8; 5000],
        );
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::DuplicatePath(path) => {
                assert_eq!(path, "dup.txt");
            }
            other => panic!("Expected DuplicatePath error, got: {:?}", other),
        }
    }

    #[test]
    fn test_overlap_violation_handling() {
        // file A: [4096..12288], file B: [8192..12288] (overlap!)
        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(b"fileA", 4096, 8192), (b"fileB", 8192, 4096)],
            &vec![0u8; 15000],
        );
        let temp = write_temp_archive(&raw);
        let res = AxicArchive::open(temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::OverlapViolation { path_a, path_b, .. } => {
                assert_eq!(path_a, "fileA");
                assert_eq!(path_b, "fileB");
            }
            other => panic!("Expected OverlapViolation error, got: {:?}", other),
        }
    }

    #[test]
    fn test_open_valid_archive() {
        // valid file A: [4096..4106], file B: [8192..8192] (zero-sized)
        let mut extra = vec![0u8; 12000];
        // write some content for file A at offset 4096 (header is 12 + 2 * 272 = 556 bytes, so 4096 is safe)
        let file_a_offset = 4096;
        let file_a_data = b"Hello VFS!";
        let extra_offset = file_a_offset - 556; // because raw builder appends extra_data directly after TOC
        extra[extra_offset..extra_offset + file_a_data.len()].copy_from_slice(file_a_data);

        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(b"fileA", file_a_offset as u64, file_a_data.len() as u64), (b"fileB", 8192, 0)],
            &extra,
        );
        let temp = write_temp_archive(&raw);
        let archive = AxicArchive::open(temp.path()).unwrap();

        assert_eq!(archive.toc.len(), 2);
        assert!(archive.toc.contains_key("fileA"));
        assert!(archive.toc.contains_key("fileB"));

        let data_a = archive.get_file("fileA").unwrap();
        assert_eq!(data_a, file_a_data);

        let data_b = archive.get_file("fileB").unwrap();
        assert_eq!(data_b, b"");

        let err = archive.get_file("nonexistent");
        assert!(err.is_err());
        match err.unwrap_err() {
            VfsError::FileNotFound(path) => assert_eq!(path, "nonexistent"),
            other => panic!("Expected FileNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_archive_pack_and_open_roundtrip() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let source_dir = dir.path();

        let file1 = source_dir.join("file1.bin");
        let file2 = source_dir.join("file2.bin");
        let file3 = source_dir.join("file3.bin");

        std::fs::write(&file1, b"File 1 content").unwrap();
        std::fs::write(&file2, vec![0xAA; 5000]).unwrap();
        std::fs::write(&file3, b"").unwrap();

        // Write the archive to a temp file outside the source directory
        let archive_temp = NamedTempFile::new().unwrap();
        let archive_path = archive_temp.path();

        pack_directory(source_dir, archive_path).unwrap();

        // Open the archive
        let archive = AxicArchive::open(archive_path).unwrap();

        // Check file count (should be 3)
        assert_eq!(archive.toc.len(), 3);

        // Check contents and offsets
        for (name, expected_data) in [
            ("file1.bin", b"File 1 content" as &[u8]),
            ("file2.bin", &vec![0xAA; 5000]),
            ("file3.bin", b""),
        ] {
            let (offset, size) = *archive.toc.get(name).expect("File must be in TOC");
            assert_eq!(offset % 4096, 0, "Offset for {} is not page aligned", name);
            assert_eq!(size, expected_data.len());

            let data = archive.get_file(name).unwrap();
            assert_eq!(data, expected_data);
        }

        // Test extraction (INV-CROSS-009)
        let extract_dir = tempdir().unwrap();
        let dest_file1 = extract_dir.path().join("extracted_file1.bin");
        archive.extract_file("file1.bin", &dest_file1).unwrap();

        let extracted_data1 = std::fs::read(&dest_file1).unwrap();
        assert_eq!(extracted_data1, b"File 1 content");
    }

    #[test]
    fn test_pack_directory_errors() {
        use tempfile::tempdir;

        // Test NotADirectory
        let temp_file = NamedTempFile::new().unwrap();
        let res = pack_directory(temp_file.path(), Path::new("dummy.axic"));
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::NotADirectory(_) => {}
            other => panic!("Expected NotADirectory, got {:?}", other),
        }

        // Test PathTooLong
        let dir = tempdir().unwrap();
        let long_name = "а".repeat(130); // 130 Cyrillic characters = 260 UTF-8 bytes (limit is 255 bytes)
        let file_path = dir.path().join(&long_name);
        std::fs::write(&file_path, b"dummy").unwrap();

        let out_temp = NamedTempFile::new().unwrap();
        let res = pack_directory(dir.path(), out_temp.path());
        assert!(res.is_err());
        match res.unwrap_err() {
            VfsError::PathTooLong(path) => {
                assert_eq!(path, long_name);
            }
            other => panic!("Expected PathTooLong, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_archive_handling() {
        // empty archive (file_count = 0)
        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[],
            &[],
        );
        let temp = write_temp_archive(&raw);
        let archive = AxicArchive::open(temp.path()).unwrap();
        assert_eq!(archive.toc.len(), 0);
    }

    #[test]
    fn test_toc_uniqueness_and_null_termination() {
        // We test null-termination extraction: a path with null bytes inside the 256-byte field
        // should be truncated at the first null byte.
        let mut path_bytes = [0u8; 256];
        path_bytes[0] = b'a';
        path_bytes[1] = b'b';
        // path_bytes[2] is 0 (null terminator)
        path_bytes[3] = b'c'; // should be ignored

        let raw = build_test_archive_raw(
            crate::constants::AXIC_MAGIC.to_le_bytes(),
            1,
            &[(&path_bytes[..], 4096, 10)],
            &vec![0u8; 5000],
        );
        let temp = write_temp_archive(&raw);
        let archive = AxicArchive::open(temp.path()).unwrap();
        assert!(archive.toc.contains_key("ab"));
        assert!(!archive.toc.contains_key("ab\0c"));
    }

    #[test]
    fn test_archive_pack_recursive() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let source_dir = dir.path();

        // Create structure:
        // source_dir/file_root.bin
        // source_dir/nested/file_nested.bin
        // source_dir/nested/deep/file_deep.bin
        let root_file = source_dir.join("file_root.bin");
        let nested_dir = source_dir.join("nested");
        let deep_dir = nested_dir.join("deep");

        std::fs::create_dir_all(&deep_dir).unwrap();

        let nested_file = nested_dir.join("file_nested.bin");
        let deep_file = deep_dir.join("file_deep.bin");

        std::fs::write(&root_file, b"root content").unwrap();
        std::fs::write(&nested_file, b"nested content").unwrap();
        std::fs::write(&deep_file, b"deep content").unwrap();

        let archive_temp = NamedTempFile::new().unwrap();
        let archive_path = archive_temp.path();

        pack_directory(source_dir, archive_path).unwrap();

        let archive = AxicArchive::open(archive_path).unwrap();

        // Check file count
        assert_eq!(archive.toc.len(), 3);

        // Check files are present with correct paths (using forward slashes)
        assert!(archive.toc.contains_key("file_root.bin"));
        assert!(archive.toc.contains_key("nested/file_nested.bin"));
        assert!(archive.toc.contains_key("nested/deep/file_deep.bin"));

        // Check content
        assert_eq!(archive.get_file("file_root.bin").unwrap(), b"root content");
        assert_eq!(archive.get_file("nested/file_nested.bin").unwrap(), b"nested content");
        assert_eq!(archive.get_file("nested/deep/file_deep.bin").unwrap(), b"deep content");
    }
}
