#![cfg(test)]

use super::*;

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // InvalidProof
fn claim_reverts_on_tampered_public_input() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let recipient = real_recipient_r0(&s.env);
    // the real proof's actual output is real_nullifier_hash(); claiming
    // with a different nullifier_hash means the pairing check is being
    // asked to verify a statement the proof doesn't attest to.
    let wrong_nullifier_hash =
        real_nullifier_hash(&s.env) + Fr::from_u256(U256::from_u32(&s.env, 1));
    let external_nullifier = real_external_nullifier_round0(&s.env);
    let proof = real_valid_proof(&s.env);

    client.claim(
        &s.circle_id,
        &recipient,
        &wrong_nullifier_hash,
        &external_nullifier,
        &proof,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // RoundNotFunded
                                                  // Ideally we'd pin pot == contribution*size - 1 (the single stroop
                                                  // short of full) as the tightest possible underfunded case. But `fund`
                                                  // only ever moves whole `contribution`-sized deposits — there's no way
                                                  // to land the pot on a non-multiple-of-contribution value through the
                                                  // public API. The tightest *reachable* underfunded state is one missing
                                                  // depositor, so that's what this test pins instead.
fn claim_reverts_when_underfunded() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    // only 4 of 5 members fund this round
    for m in s.members.iter().take(4) {
        client.fund(&s.circle_id, m);
    }

    let recipient = real_recipient_r0(&s.env);
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
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // RoundNotFunded
fn claim_immediately_after_round_advance_reverts() {
    // Regression guard: after a successful claim, pot must reset to 0 and
    // round 2 must require its own fresh funding — not silently inherit
    // round 1's now-stale "fully funded" state.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let recipient = real_recipient_r0(&s.env);
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

    let circle = client.get_circle(&s.circle_id);
    assert_eq!(circle.pot, 0);
    assert_eq!(circle.round, 1);

    // No one has funded round 1 yet — this must revert with RoundNotFunded,
    // not pay out against a stale/leftover pot value.
    let recipient2 = Address::generate(&s.env);
    client.claim(
        &s.circle_id,
        &recipient2,
        &nullifier_hash,
        &external_nullifier,
        &proof,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // AlreadyClaimed
fn second_claim_with_same_nullifier_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let nullifier_hash = real_nullifier_hash(&s.env);
    let proof = real_valid_proof(&s.env);

    // round 0: claim succeeds and marks the nullifier used
    let recipient_a = real_recipient_r0(&s.env);
    let external_nullifier_0 = real_external_nullifier_round0(&s.env);
    client.claim(
        &s.circle_id,
        &recipient_a,
        &nullifier_hash,
        &external_nullifier_0,
        &proof,
    );

    // top up and fund round 1 fully, then try to reuse the exact same
    // nullifier_hash from round 0. It's rejected by the nullifier map
    // before the (real, but now mismatched-round) proof would even be
    // checked, so reusing `proof` here is fine.
    let token_admin_client = token::StellarAssetClient::new(&s.env, &s.token);
    for m in s.members.iter() {
        token_admin_client.mint(m, &s.contribution);
        client.fund(&s.circle_id, m);
    }
    let recipient_b = Address::generate(&s.env);
    let external_nullifier_1 = expected_external_nullifier(&s.env, s.circle_id, 1);
    client.claim(
        &s.circle_id,
        &recipient_b,
        &nullifier_hash,
        &external_nullifier_1,
        &proof,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // WrongRoundTag
fn claim_reverts_on_stale_round_tag() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let recipient = real_recipient_r0(&s.env);
    let nullifier_hash = real_nullifier_hash(&s.env);
    // wrong: this circle is still on round 0, but we tag the proof for round 1
    let external_nullifier = expected_external_nullifier(&s.env, s.circle_id, 1);
    let proof = real_valid_proof(&s.env);

    client.claim(
        &s.circle_id,
        &recipient,
        &nullifier_hash,
        &external_nullifier,
        &proof,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // CircleNotFound
fn claim_unknown_circle_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let recipient = real_recipient_r0(&s.env);
    client.claim(
        &999u64,
        &recipient,
        &real_nullifier_hash(&s.env),
        &real_external_nullifier_round0(&s.env),
        &real_valid_proof(&s.env),
    );
}

#[test]
fn claim_works_on_fully_funded_round_after_cap() {
    // Companion to sixth_fund_on_full_round_reverts: five funds reach the
    // cap exactly, claim still pays out (over-funding never mutated state).
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let token_client = token::Client::new(&s.env, &s.token);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }
    assert_eq!(
        client.get_circle(&s.circle_id).pot,
        s.contribution * (s.size as i128)
    );

    let recipient = real_recipient_r0(&s.env);
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
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // CircleCancelled
fn claim_after_cancel_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }
    client.cancel_circle(&s.circle_id);

    let recipient = real_recipient_r0(&s.env);
    client.claim(
        &s.circle_id,
        &recipient,
        &real_nullifier_hash(&s.env),
        &real_external_nullifier_round0(&s.env),
        &real_valid_proof(&s.env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // CircleCancelled
fn claim_after_zero_contributor_cancel_reverts() {
    // Companion to claim_after_cancel_reverts, starting from the empty state
    // (no members funded before the cancel).
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    client.cancel_circle(&s.circle_id);

    let recipient = real_recipient_r0(&s.env);
    client.claim(
        &s.circle_id,
        &recipient,
        &real_nullifier_hash(&s.env),
        &real_external_nullifier_round0(&s.env),
        &real_valid_proof(&s.env),
    );
}
