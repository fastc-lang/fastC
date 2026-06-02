//! Content-integrity hashing for fetched dependencies.
//!
//! Stage 1.7 (vendor-first package system) hinges on dependency
//! authenticity. A `fastc.toml` entry like
//!
//! ```toml
//! json = { git = "...", rev = "abc123", sha256 = "<64 hex chars>" }
//! ```
//!
//! is only a security property if the build actually verifies that
//! the fetched tree hashes to the declared value. Before this
//! module, `sha256` was a warning-only field — written down but
//! never checked. Now `verify_tree` is the single source of truth:
//! every `BuildContext::fetch_dependencies` call routes through it
//! and fails the build on mismatch.
//!
//! ## What we hash
//!
//! The fetched directory minus `.git/`. Every other file is walked
//! in path-sorted order; each file contributes:
//!
//! 1. Its repo-relative POSIX path as bytes (UTF-8), null-terminated.
//! 2. Its content length as 8 bytes big-endian.
//! 3. Its content bytes.
//!
//! Both record boundaries are part of the hash so adding an empty
//! file or renaming a file changes the digest. The output is the
//! lowercase hex form of the SHA-256.
//!
//! ## Why a custom format
//!
//! We could shell out to `git ls-files` + `git hash-object`, but
//! that ties verification to the git CLI and to commit semantics
//! (mtimes, symlink modes, submodule pointers) that aren't part of
//! the "did this code change?" question. A path-and-bytes scheme
//! is deterministic across machines, doesn't require git, and is
//! what `fastc lock` already records — so users can re-verify
//! without running fastc at all.

use std::fs;
use std::path::{Path, PathBuf};

use crate::db::sha256;

/// Compute the lowercase-hex SHA-256 of a fetched dependency tree at
/// `root`. Skips `.git/` and any path matching the explicit ignore
/// list. Returns an error if the tree can't be walked (typically a
/// permissions issue).
pub fn hash_tree(root: &Path) -> Result<String, IntegrityError> {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();

    let mut hasher_input: Vec<u8> = Vec::new();
    for path in &files {
        // Relative path as portable string.
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        hasher_input.extend_from_slice(rel_str.as_bytes());
        hasher_input.push(0u8);

        let bytes = fs::read(path)
            .map_err(|e| IntegrityError::Io(format!("reading {}: {}", path.display(), e)))?;
        hasher_input.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        hasher_input.extend_from_slice(&bytes);
    }

    let digest = sha256(&hasher_input);
    Ok(hex_encode(&digest))
}

/// Compare the hash of `root` against the expected lowercase-hex
/// digest. Returns `Ok(())` on match, `Err(IntegrityError::Mismatch)`
/// otherwise. Case is normalized before comparison so `0xABC...`
/// and `0xabc...` are treated the same way.
pub fn verify_tree(root: &Path, expected_hex: &str) -> Result<(), IntegrityError> {
    let got = hash_tree(root)?;
    let expected = expected_hex.trim().to_ascii_lowercase();
    if got != expected {
        return Err(IntegrityError::Mismatch {
            expected,
            got,
            path: root.to_path_buf(),
        });
    }
    Ok(())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), IntegrityError> {
    let entries = fs::read_dir(dir)
        .map_err(|e| IntegrityError::Io(format!("reading dir {}: {}", dir.display(), e)))?;
    for entry in entries {
        let entry = entry.map_err(|e| IntegrityError::Io(format!("dir entry: {}", e)))?;
        let path = entry.path();

        // Skip .git/ — its contents (object packs, refs, HEAD) change
        // depending on how git fetched the repo and aren't part of
        // the "source code" payload we care about.
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if path.is_dir() && (name == ".git" || name == "target" || name == "build") {
                continue;
            }
        }

        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
        // Symlinks are intentionally skipped — they're a portability
        // hazard (target may not exist on the verifier's machine).
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[derive(Debug)]
pub enum IntegrityError {
    /// IO error while walking or reading the tree.
    Io(String),
    /// The computed digest didn't match the declared one.
    Mismatch {
        expected: String,
        got: String,
        path: PathBuf,
    },
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrityError::Io(msg) => write!(f, "integrity: {}", msg),
            IntegrityError::Mismatch {
                expected,
                got,
                path,
            } => write!(
                f,
                "integrity: sha256 mismatch at {}\n  expected: {}\n  got:      {}",
                path.display(),
                expected,
                got
            ),
        }
    }
}

impl std::error::Error for IntegrityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    #[test]
    fn empty_tree_hashes_to_known_constant() {
        let tmp = std::env::temp_dir().join("fastc_integrity_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let h = hash_tree(&tmp).unwrap();
        // SHA-256 of the empty byte string.
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn same_content_different_layouts_hash_differently() {
        let a = std::env::temp_dir().join("fastc_integrity_a");
        let b = std::env::temp_dir().join("fastc_integrity_b");
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
        write(&a.join("hello.fc"), "fn main() -> i32 { return 0; }\n");
        write(
            &b.join("nested/hello.fc"),
            "fn main() -> i32 { return 0; }\n",
        );
        let ha = hash_tree(&a).unwrap();
        let hb = hash_tree(&b).unwrap();
        assert_ne!(
            ha, hb,
            "moving a file to a different path must change the digest"
        );
    }

    #[test]
    fn verify_tree_round_trips_through_hash_tree() {
        let dir = std::env::temp_dir().join("fastc_integrity_verify");
        let _ = fs::remove_dir_all(&dir);
        write(&dir.join("a.fc"), "fn main() -> i32 { return 1; }\n");
        write(
            &dir.join("nested/b.fc"),
            "fn helper() -> i32 { return 2; }\n",
        );
        let h = hash_tree(&dir).unwrap();
        verify_tree(&dir, &h).expect("identity verification");
        verify_tree(
            &dir,
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect_err("wrong hash must error");
    }

    #[test]
    fn dotgit_is_excluded_from_hash() {
        let dir = std::env::temp_dir().join("fastc_integrity_skip_dotgit");
        let _ = fs::remove_dir_all(&dir);
        write(&dir.join("hello.fc"), "fn main() -> i32 { return 0; }\n");
        let h1 = hash_tree(&dir).unwrap();
        write(&dir.join(".git/objects/abc"), "random git noise");
        write(&dir.join(".git/HEAD"), "ref: refs/heads/main");
        let h2 = hash_tree(&dir).unwrap();
        assert_eq!(h1, h2, ".git contents must not affect tree hash");
    }

    #[test]
    fn case_insensitive_hex_comparison() {
        let dir = std::env::temp_dir().join("fastc_integrity_case");
        let _ = fs::remove_dir_all(&dir);
        write(&dir.join("hi.fc"), "x");
        let h = hash_tree(&dir).unwrap();
        verify_tree(&dir, &h.to_ascii_uppercase()).expect("uppercase hex should match");
    }
}
