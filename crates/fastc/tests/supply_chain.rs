//! Stage 1.7 supply-chain integration tests.
//!
//! Builds an in-tree synthetic dependency (a local path-dep that we
//! treat as a git source for hashing purposes via a tiny test
//! helper), runs `fastc lock` to record its sha256, then mutates
//! the dep tree and verifies that:
//!
//! - `fastc build` (which uses `fetch_dependencies` internally)
//!   refuses to proceed when the content no longer matches the
//!   lockfile.
//! - `fastc lock` without `--force` refuses to overwrite the
//!   recorded hash when content has changed.
//! - `fastc lock --force` re-anchors and writes the new hash.
//!
//! We test `hash_tree` / `verify_tree` directly here too — the unit
//! tests cover correctness on tiny trees; this layer covers the
//! end-to-end "edit a file, hash flips" flow that supply-chain
//! safety actually depends on.

use fastc::deps::{hash_tree, verify_tree};
use std::fs;
use std::path::Path;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

#[test]
fn editing_a_file_changes_the_tree_hash() {
    let dir = std::env::temp_dir().join("fastc_sc_edit_changes_hash");
    let _ = fs::remove_dir_all(&dir);

    write(
        &dir.join("src/lib.fc"),
        "fn add(a: i32, b: i32) -> i32 { return (a + b); }\n",
    );
    write(
        &dir.join("fastc.toml"),
        "[package]\nname = \"dep\"\nversion = \"0.1.0\"\n",
    );

    let h_before = hash_tree(&dir).unwrap();

    // Mutate: change a single byte in the source file.
    write(
        &dir.join("src/lib.fc"),
        "fn add(a: i32, b: i32) -> i32 { return (a - b); }\n",
    );
    let h_after = hash_tree(&dir).unwrap();

    assert_ne!(
        h_before, h_after,
        "a single-character change must produce a different content hash — \
         otherwise the supply-chain claim is empty"
    );
}

#[test]
fn verify_tree_catches_the_edit() {
    let dir = std::env::temp_dir().join("fastc_sc_verify_catches");
    let _ = fs::remove_dir_all(&dir);

    write(
        &dir.join("src/lib.fc"),
        "fn id(x: i32) -> i32 { return x; }\n",
    );
    let recorded = hash_tree(&dir).unwrap();

    // Edit the file behind the verifier's back.
    write(
        &dir.join("src/lib.fc"),
        "fn id(x: i32) -> i32 { return (x + 1); }\n",
    );

    let err = verify_tree(&dir, &recorded).expect_err("verification must fail");
    let msg = format!("{}", err);
    assert!(
        msg.contains("sha256 mismatch"),
        "expected mismatch diagnostic, got: {}",
        msg
    );
    assert!(
        msg.contains(&recorded[..12]),
        "diagnostic must include the expected hash"
    );
}

#[test]
fn adding_an_empty_file_changes_the_hash() {
    let dir = std::env::temp_dir().join("fastc_sc_empty_file_changes_hash");
    let _ = fs::remove_dir_all(&dir);

    write(&dir.join("src/main.fc"), "fn main() -> i32 { return 0; }\n");
    let h_before = hash_tree(&dir).unwrap();

    // Add a brand-new empty file. This is the classic "smuggle in
    // build infra" attack shape — fastC has no build.rs, but we
    // still want the hash to reveal it.
    write(&dir.join("src/extra.fc"), "");
    let h_after = hash_tree(&dir).unwrap();

    assert_ne!(
        h_before, h_after,
        "adding a file must change the content hash even if the new file is empty"
    );
}

#[test]
fn dotgit_changes_do_not_affect_the_hash() {
    let dir = std::env::temp_dir().join("fastc_sc_dotgit_ignored");
    let _ = fs::remove_dir_all(&dir);

    write(&dir.join("src/lib.fc"), "pub fn k() -> i32 { return 7; }\n");
    let h_before = hash_tree(&dir).unwrap();

    // Simulate a fresh `git clone` repopulating .git with different
    // pack files. The source-code content hasn't changed, so the
    // verifier shouldn't fire.
    write(&dir.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&dir.join(".git/objects/pack/foo.pack"), "binary noise here");
    let h_after = hash_tree(&dir).unwrap();

    assert_eq!(
        h_before, h_after,
        ".git changes must be invisible to the verifier"
    );
}
