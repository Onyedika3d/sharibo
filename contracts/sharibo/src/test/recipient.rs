#![cfg(test)]

use super::*;

// ---- Mock-verifier recipient tests (restored; see PR summary) ----
//
// Simulate the contract's claim-time checks with a test-local mock verifier
// that returns `true` only when the public input `recipientHash` matches the
// original recipient's computed hash — the acceptance criteria from before
// the recipientHash plumbing was regenerated into real fixtures (issue #266).

fn simulate_claim_with_mock(
    env: &Env,
    circle: &Circle,
    recipient: &Address,
    expected_recipient_hash: Fr,
    nullifier_hash: Fr,
    external_nullifier: Fr,
    proof: &Proof,
) {
    // Basic precondition checks mirrored from `claim`.
    if circle.pot != circle.contribution * (circle.size as i128) {
        panic_with_error!(env, Error::RoundNotFunded);
    }

    // Compute the recipientHash the contract would derive from `recipient`.
    let recipient_hash = Contract::compute_recipient_hash(env, recipient);

    // Build public inputs in the circuit-expected order.
    let mut public_inputs: Vec<Fr> = Vec::new(env);
    public_inputs.push_back(nullifier_hash.clone());
    public_inputs.push_back(circle.root.clone());
    public_inputs.push_back(external_nullifier.clone());
    public_inputs.push_back(recipient_hash.clone());

    // Mock verifier: accept iff the public-signal recipientHash equals the
    // expected recipient hash that the original proof was generated for.
    let mock_ok = {
        // public_inputs[3]
        let pi_recipient = public_inputs.get(3).unwrap();
        pi_recipient == expected_recipient_hash
    };

    if !mock_ok {
        panic_with_error!(env, Error::InvalidProof);
    }
}

#[test]
fn mock_accepts_original_recipient() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(3, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let circle = client.get_circle(&s.circle_id);
    let recipient_a = Address::generate(&s.env);
    let nullifier_hash = real_nullifier_hash(&s.env);
    let external_nullifier = real_external_nullifier_round0(&s.env);
    let proof = real_valid_proof(&s.env);

    // The 'expected' recipient hash is computed for recipient_a (what the
    // real off-chain prover would have used when generating the fixture).
    let expected_recipient_hash = Contract::compute_recipient_hash(&s.env, &recipient_a);

    // Should not panic: mock verifier accepts when recipient matches.
    simulate_claim_with_mock(
        &s.env,
        &circle,
        &recipient_a,
        expected_recipient_hash,
        nullifier_hash,
        external_nullifier,
        &proof,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // InvalidProof
fn mock_replayed_proof_with_different_recipient_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(3, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let circle = client.get_circle(&s.circle_id);
    let recipient_a = Address::generate(&s.env);
    let recipient_b = Address::generate(&s.env);
    let nullifier_hash = real_nullifier_hash(&s.env);
    let external_nullifier = real_external_nullifier_round0(&s.env);
    let proof = real_valid_proof(&s.env);

    // The original proof was generated for recipient_a.
    let expected_recipient_hash = Contract::compute_recipient_hash(&s.env, &recipient_a);

    // Now attempt to 'claim' but with a different recipient (recipient_b).
    // The mock verifier should reject and the helper simulates the
    // contract's panic with Error::InvalidProof.
    simulate_claim_with_mock(
        &s.env,
        &circle,
        &recipient_b,
        expected_recipient_hash,
        nullifier_hash,
        external_nullifier,
        &proof,
    );
}
