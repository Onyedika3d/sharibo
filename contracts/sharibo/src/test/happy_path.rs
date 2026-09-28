#![cfg(test)]

use super::*;

#[test]
fn happy_path_round_pays_out_and_advances() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let token_client = token::Client::new(&s.env, &s.token);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let circle = client.get_circle(&s.circle_id);
    assert_eq!(circle.pot, s.contribution * (s.size as i128));

    let recipient = real_recipient_r0(&s.env); // fixed fixture recipient bound to real_valid_proof
    let nullifier_hash = real_nullifier_hash(&s.env);
    let external_nullifier = real_external_nullifier_round0(&s.env);
    let proof = real_valid_proof(&s.env);

    client.claim(
        &s.circle_id,
        &recipient,
        &nullifier_hash,
        &external_nullifier,
        &proof,
    );

    assert_eq!(
        token_client.balance(&recipient),
        s.contribution * (s.size as i128)
    );
    assert_eq!(token_client.balance(&s.client_id), 0);

    let circle_after = client.get_circle(&s.circle_id);
    assert_eq!(circle_after.pot, 0);
    assert_eq!(circle_after.round, 1);
}

// ---- Issue #91: current multi-round semantics ----
//
// This is the definitive answer to "can the same identity claim two
// consecutive rounds today?" — YES. `nullifierHash = Poseidon(identityNullifier,
// externalNullifier)` and externalNullifier is derived from `round`, so the
// same identity produces a *different* nullifierHash each round, and the
// contract's nullifier map is keyed per (circle_id, nullifier_hash) with no
// round-independent identity tracking. Nothing here is a bug in the code
// tested elsewhere in this file (WrongRoundTag/AlreadyClaimed both still work
// correctly per-round) — it's a real gap: nothing currently stops one member
// from claiming every single round of a cycle. See docs/ for the proposed fix.
#[test]
fn same_identity_can_claim_two_consecutive_rounds() {
    let env = Env::default();
    env.mock_all_auths();

    let s = TestCircle::new(1, 100)
        .with_vk(round_reuse_verification_key(&env))
        .build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let token_admin_client = token::StellarAssetClient::new(&s.env, &s.token);
    let token_client = token::Client::new(&s.env, &s.token);

    // ---- round 0: fund and claim with the real identity ----
    let funder = Address::generate(&env);
    token_admin_client.mint(&funder, &100i128);
    client.fund(&s.circle_id, &funder);

    let nullifier_hash_r0 = real_nullifier_hash(&env);
    let external_nullifier_r0 = real_external_nullifier_round0(&env);
    let proof_r0 = round_reuse_proof_round0(&env);

    assert!(!client.has_claimed(&s.circle_id, &nullifier_hash_r0));
    let recipient_r0 = real_recipient_r0(&env);
    client.claim(
        &s.circle_id,
        &recipient_r0,
        &nullifier_hash_r0,
        &external_nullifier_r0,
        &proof_r0,
    );
    assert!(client.has_claimed(&s.circle_id, &nullifier_hash_r0));
    assert_eq!(token_client.balance(&recipient_r0), 100i128);

    let circle = client.get_circle(&s.circle_id);
    assert_eq!(circle.round, 1);
    assert_eq!(circle.pot, 0);

    // ---- round 1: fund again, then claim again — same identity, no error ----
    token_admin_client.mint(&funder, &100i128);
    client.fund(&s.circle_id, &funder);

    let nullifier_hash_r1 = round_reuse_nullifier_hash_round1(&env);
    let external_nullifier_r1 = expected_external_nullifier(&env, s.circle_id, 1);
    let proof_r1 = round_reuse_proof_round1(&env);

    // Different round -> different nullifierHash for the SAME identity, so
    // it reads as "never claimed" even though this identity already claimed
    // round 0 above.
    assert_ne!(nullifier_hash_r0, nullifier_hash_r1);
    assert!(!client.has_claimed(&s.circle_id, &nullifier_hash_r1));

    let recipient_r1 = real_recipient_r1(&env);
    client.claim(
        &s.circle_id,
        &recipient_r1,
        &nullifier_hash_r1,
        &external_nullifier_r1,
        &proof_r1,
    );

    // The claim succeeded: no RoundNotFunded/WrongRoundTag/AlreadyClaimed/
    // InvalidProof panic. Same identity, two rounds, two payouts.
    assert!(client.has_claimed(&s.circle_id, &nullifier_hash_r1));
    assert_eq!(token_client.balance(&recipient_r1), 100i128);
    assert_eq!(client.get_circle(&s.circle_id).round, 2);
}
