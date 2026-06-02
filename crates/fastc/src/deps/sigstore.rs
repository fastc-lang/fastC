//! Sigstore bundle verification via `cosign verify-blob`.
//!
//! Stage 1.7 (vendor-first + Sigstore). A dependency entry can carry
//! a `sigstore = "<path>"` field pointing at a `.sigstore.json`
//! bundle that signs the dep's content hash. When the user runs
//! `fastc build`, this module:
//!
//! 1. Checks whether `cosign` is on PATH. If not, prints a warning
//!    (`sigstore: cosign not installed — skipping signature check`)
//!    and returns `Ok(())`. We don't refuse builds on missing tools
//!    because fastC ships into environments (embedded SDKs, isolated
//!    CI runners) where installing cosign isn't always trivial; we'd
//!    rather warn loudly than block fast feedback.
//!
//! 2. With cosign present, shells out to:
//!
//!    ```text
//!    cosign verify-blob \
//!        --bundle <path>/<bundle> \
//!        --new-bundle-format \
//!        --certificate-identity-regexp <identity_regexp> \
//!        --certificate-oidc-issuer <oidc_issuer> \
//!        <signed-payload>
//!    ```
//!
//!    The signed payload is the content sha256 of the fetched tree
//!    (so what's signed is exactly what fastC just verified
//!    structurally). Identity / OIDC issuer come from the manifest
//!    when set, falling back to fastc-core defaults.
//!
//! 3. Non-zero exit from cosign fails the build with the cosign
//!    diagnostic surfaced verbatim.
//!
//! This is the smallest meaningful integration. Future work:
//! - Cache verification results so repeated builds don't re-shell.
//! - Support inline bundles (publisher pastes JSON directly into
//!   fastc.toml) for users who don't want a separate file.
//! - Trust-on-first-use of the certificate identity, recorded into
//!   fastc.lock alongside sha256.

use std::path::Path;
use std::process::Command;

/// Default certificate-identity regexp for `fastc-core` packages.
/// Matches any GitHub Actions workflow under Skelf-Research's
/// `fastc-core-*` repos. Overridable via the manifest's optional
/// `sigstore_identity` field (added separately when the field exists).
pub const DEFAULT_IDENTITY_REGEXP: &str =
    "^https://github.com/Skelf-Research/fastc-core-[a-z0-9_-]+/\\.github/workflows/.+$";

/// Default OIDC issuer expected on fastc-core signatures. GitHub
/// Actions tokens are issued by this URL when cosign keyless signs.
pub const DEFAULT_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// Outcome of attempting to verify a sigstore bundle for a dep.
#[derive(Debug)]
pub enum SigstoreOutcome {
    /// `cosign verify-blob` succeeded.
    Verified,
    /// `cosign` isn't on PATH — verification skipped with a warning.
    Skipped { reason: String },
    /// cosign returned non-zero. The build should fail.
    Failed { stdout: String, stderr: String },
    /// Sigstore field wasn't set on this dep — nothing to verify.
    NotConfigured,
}

/// Verify the sigstore bundle declared by a dep's manifest entry.
///
/// `bundle_relative` is the `sigstore = "..."` value, interpreted
/// relative to the fetched dep's root. `signed_payload` is what we
/// expect the bundle to sign — for fastC that's the content sha256
/// of the dep tree (lowercase hex), which the build already
/// computed independently via `deps::hash_tree`.
pub fn verify(
    dep_root: &Path,
    bundle_relative: Option<&str>,
    signed_payload: &str,
) -> SigstoreOutcome {
    let Some(bundle_path) = bundle_relative else {
        return SigstoreOutcome::NotConfigured;
    };
    let bundle = dep_root.join(bundle_path);
    if !bundle.exists() {
        return SigstoreOutcome::Failed {
            stdout: String::new(),
            stderr: format!("sigstore bundle not found at {}", bundle.display()),
        };
    }

    if !cosign_on_path() {
        return SigstoreOutcome::Skipped {
            reason:
                "cosign not on PATH (install from https://docs.sigstore.dev/cosign/installation)"
                    .to_string(),
        };
    }

    // Write the signed payload to a temp file so cosign can read it.
    let tmp = std::env::temp_dir().join(format!("fastc-sigstore-payload-{}", std::process::id()));
    if let Err(e) = std::fs::write(&tmp, signed_payload.as_bytes()) {
        return SigstoreOutcome::Failed {
            stdout: String::new(),
            stderr: format!("failed to write signed payload: {}", e),
        };
    }

    let output = match Command::new("cosign")
        .args(["verify-blob", "--bundle"])
        .arg(&bundle)
        .args([
            "--new-bundle-format",
            "--certificate-identity-regexp",
            DEFAULT_IDENTITY_REGEXP,
            "--certificate-oidc-issuer",
            DEFAULT_OIDC_ISSUER,
        ])
        .arg(&tmp)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return SigstoreOutcome::Failed {
                stdout: String::new(),
                stderr: format!("failed to invoke cosign: {}", e),
            };
        }
    };

    let _ = std::fs::remove_file(&tmp);

    if output.status.success() {
        SigstoreOutcome::Verified
    } else {
        SigstoreOutcome::Failed {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

fn cosign_on_path() -> bool {
    let Some(path_env) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_env) {
        if dir.join("cosign").is_file() {
            return true;
        }
        if dir.join("cosign.exe").is_file() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_sigstore_field_is_not_configured() {
        let dir = std::env::temp_dir().join("fastc_sig_notcfg");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let out = verify(&dir, None, "abc");
        assert!(matches!(out, SigstoreOutcome::NotConfigured));
    }

    #[test]
    fn missing_bundle_file_fails() {
        let dir = std::env::temp_dir().join("fastc_sig_nobundle");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let out = verify(&dir, Some("vendor/foo.sigstore.json"), "abc");
        assert!(matches!(out, SigstoreOutcome::Failed { .. }));
    }

    #[test]
    fn defaults_are_fastc_core_shaped() {
        assert!(DEFAULT_IDENTITY_REGEXP.contains("fastc-core"));
        assert_eq!(
            DEFAULT_OIDC_ISSUER,
            "https://token.actions.githubusercontent.com"
        );
    }
}
