#![cfg(test)]

use super::*;
use soroban_sdk::xdr::ToXdr;
use std::path::PathBuf;
use std::string::{String, ToString};

// ---------------------------------------------------------------------------
// XDR wire-format goldens (issue #326, restored for #479)
//
// Serialise each contracttype to XDR via soroban_sdk::xdr::ToXdr, base64-
// encode with an inline RFC-4648 encoder (no new crate), and compare against
// a committed .b64 golden file. Adding a field, reordering fields, or
// changing a type silently changes the XDR wire format; the failure appears
// as a decode error in the browser rather than a test failure. Committed
// goldens make that silent failure loud.
//
// UPDATE_GOLDEN=1 cargo test -p sharibo xdr_golden regenerates all golden
// files in place.
//
// The module was lost in the #235 test-split refactor (a4fae0d) which
// reverted test.rs to a single file; the golden .b64 files under
// test_snapshots/xdr_goldens/ were orphaned alongside it.
// ---------------------------------------------------------------------------

// Schema version — bump this (and regenerate the goldens) whenever the
// XDR wire format changes intentionally.
const SCHEMA_VERSION: u32 = 1;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Base64-encode a soroban `Bytes` value using the standard alphabet
/// with padding, matching what `btoa` / `Buffer#toString('base64')` emit
/// on the TypeScript side.
fn to_base64(bytes: &soroban_sdk::Bytes) -> String {
    // Pull the raw byte slice out of the soroban `Bytes` container.
    let raw: std::vec::Vec<u8> = bytes.iter().collect();
    base64_encode(&raw)
}

/// Minimal base64 encoder — avoids pulling in an extra crate.
/// Uses the standard RFC 4648 alphabet with `=` padding.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as u32;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as u32
        } else {
            0
        };
        out.push(ALPHABET[((b0 >> 2) & 0x3f) as usize] as char);
        out.push(ALPHABET[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize] as char);
        if i + 1 < input.len() {
            out.push(ALPHABET[(((b1 << 2) | (b2 >> 6)) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

/// Absolute path to the XDR goldens directory.
fn goldens_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is set by the test harness to the crate root.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set — run via `cargo test`");
    PathBuf::from(manifest)
        .join("test_snapshots")
        .join("xdr_goldens")
}

/// Read a committed golden (returns `None` if the file does not exist yet).
fn read_golden(name: &str) -> Option<String> {
    let path = goldens_dir().join(name);
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Write (or overwrite) a golden file, creating the directory if needed.
fn write_golden(name: &str, content: &str) {
    let dir = goldens_dir();
    std::fs::create_dir_all(&dir).expect("could not create xdr_goldens directory");
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("could not write golden {name}: {e}"));
    std::println!("  [UPDATE_GOLDEN] wrote {}", path.display());
}

/// Core assertion: compare `actual` base64 against the committed golden.
///
/// * With `UPDATE_GOLDEN=1`: writes the new golden and passes.
/// * Without it: asserts byte-for-byte equality with the prescribed
///   failure message.
fn assert_golden(name: &str, actual: &str) {
    let updating = std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1");
    if updating {
        write_golden(name, actual);
        return;
    }
    match read_golden(name) {
        None => {
            panic!(
                "\nGolden file `{name}` does not exist.\n\
                 Run `UPDATE_GOLDEN=1 cargo test -p sharibo xdr_golden` to generate it,\n\
                 then commit the new file alongside this test.\n"
            );
        }
        Some(expected) => {
            assert_eq!(
                actual,
                expected.as_str(),
                "\n\
                 ┌─────────────────────────────────────────────────────────────┐\n\
                 │  XDR wire format changed — golden `{name}` no longer matches │\n\
                 │                                                             │\n\
                 │  The storage layout changed — bump schema_version and       │\n\
                 │  update the golden deliberately:                            │\n\
                 │                                                             │\n\
                 │    1. Bump SCHEMA_VERSION in test.rs (currently {SCHEMA_VERSION}).      │\n\
                 │    2. Run: UPDATE_GOLDEN=1 cargo test -p sharibo xdr_golden │\n\
                 │    3. Also update packages/client/src/contract.test.ts      │\n\
                 │       (the TypeScript golden decoder) and its snapshot.     │\n\
                 │    4. Commit all three changes together.                    │\n\
                 └─────────────────────────────────────────────────────────────┘\n"
            );
        }
    }
}

// -----------------------------------------------------------------------
// Deterministic Circle fixture
// -----------------------------------------------------------------------
//
// We construct a Circle with every field set to a fixed, human-readable
// value so that the XDR golden is stable across runs and machines.
//
// The admin and token addresses use `Address::from_str` on well-known
// testnet G-addresses rather than `Address::generate` (which is random)
// so the serialised bytes are deterministic.
//
// VK and Proof come from the same real_* fixtures used by the rest of
// the test suite — they are committed constants derived from the actual
// Phase 1 trusted-setup ceremony.

const GOLDEN_ADMIN: &str = "GAAQEAYEAUDAOCAJBIFQYDIOB4IBCEQTCQKRMFYYDENBWHA5DYPSABOV";
const GOLDEN_TOKEN: &str = "GABAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEJXA";

fn golden_circle(env: &Env) -> Circle {
    // Parse the two well-known addresses deterministically.
    let admin = Address::from_str(env, GOLDEN_ADMIN);
    let token = Address::from_str(env, GOLDEN_TOKEN);

    Circle {
        schema_version: 2,
        admin,
        token,
        root: real_root(env),
        contribution: 1_000_000i128,
        size: 5u32,
        round: 0u32,
        pot: 0i128,
        vk: real_verification_key(env),
        contributors: soroban_sdk::Vec::new(env),
        nullifiers: soroban_sdk::Vec::new(env),
        cancelled: false,
        round_deadline_ledgers: 0u32,
        round_started_ledger: 0u32,
        fee_bps: 0u32,
        fee_recipient: Address::from_str(env, GOLDEN_ADMIN),
    }
}

// -----------------------------------------------------------------------
// Test: Circle golden
// -----------------------------------------------------------------------

#[test]
fn xdr_golden_circle() {
    let env = Env::default();
    let circle = golden_circle(&env);
    let xdr_bytes = circle.to_xdr(&env);
    let b64 = to_base64(&xdr_bytes);
    assert_golden("circle.v1.b64", &b64);
}

// -----------------------------------------------------------------------
// Test: VerificationKey golden
// -----------------------------------------------------------------------

#[test]
fn xdr_golden_verification_key() {
    let env = Env::default();
    let vk = real_verification_key(&env);
    let xdr_bytes = vk.to_xdr(&env);
    let b64 = to_base64(&xdr_bytes);
    assert_golden("verification_key.v1.b64", &b64);
}

// -----------------------------------------------------------------------
// Test: Proof golden
// -----------------------------------------------------------------------

#[test]
fn xdr_golden_proof() {
    let env = Env::default();
    let proof = real_valid_proof(&env);
    let xdr_bytes = proof.to_xdr(&env);
    let b64 = to_base64(&xdr_bytes);
    assert_golden("proof.v1.b64", &b64);
}

// -----------------------------------------------------------------------
// Test: round-trip (XDR → deserialise → re-serialise == original)
// -----------------------------------------------------------------------
//
// This is an independent sanity check: the base64 golden tests above
// verify byte-for-byte stability.  This round-trip test verifies that
// the XDR encoding is also self-consistent (i.e. FromXdr(ToXdr(x)) == x).
// Reordering fields breaks the golden *and* this round-trip, giving a
// second signal that the wire format changed.

#[test]
fn xdr_circle_round_trips() {
    use soroban_sdk::xdr::FromXdr;

    let env = Env::default();
    let circle = golden_circle(&env);
    let xdr_bytes = circle.clone().to_xdr(&env);
    let recovered = Circle::from_xdr(&env, &xdr_bytes)
        .expect("Circle::from_xdr failed — the XDR layout is inconsistent");

    // Compare field-by-field so failures point at the specific field.
    assert_eq!(recovered.contribution, circle.contribution, "contribution");
    assert_eq!(recovered.size, circle.size, "size");
    assert_eq!(recovered.round, circle.round, "round");
    assert_eq!(recovered.pot, circle.pot, "pot");
    assert_eq!(recovered.cancelled, circle.cancelled, "cancelled");
    // Fr/G1Affine/G2Affine don't implement PartialEq directly, so we
    // re-serialise and compare the bytes as a proxy for equality.
    assert_eq!(
        recovered.root.to_xdr(&env),
        circle.root.to_xdr(&env),
        "root"
    );
}

#[test]
fn xdr_proof_round_trips() {
    use soroban_sdk::xdr::FromXdr;

    let env = Env::default();
    let proof = real_valid_proof(&env);
    let xdr_bytes = proof.clone().to_xdr(&env);
    let recovered = Proof::from_xdr(&env, &xdr_bytes).expect("Proof::from_xdr failed");

    assert_eq!(recovered.a.to_xdr(&env), proof.a.to_xdr(&env), "proof.a");
    assert_eq!(recovered.b.to_xdr(&env), proof.b.to_xdr(&env), "proof.b");
    assert_eq!(recovered.c.to_xdr(&env), proof.c.to_xdr(&env), "proof.c");
}

#[test]
fn xdr_verification_key_round_trips() {
    use soroban_sdk::xdr::FromXdr;

    let env = Env::default();
    let vk = real_verification_key(&env);
    let xdr_bytes = vk.clone().to_xdr(&env);
    let recovered =
        VerificationKey::from_xdr(&env, &xdr_bytes).expect("VerificationKey::from_xdr failed");

    assert_eq!(
        recovered.alpha.to_xdr(&env),
        vk.alpha.to_xdr(&env),
        "vk.alpha"
    );
    assert_eq!(vk.ic.len(), recovered.ic.len(), "vk.ic length");
}
