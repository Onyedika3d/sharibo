#![cfg(test)]

use super::*;

// ---- Issue #252: protocol fees ----

// Requires a successful claim: the proof must verify against the committed
// vk AND its recipientHash public input must match the payout address
// (issue #266/#275) — satisfied by the regenerated fixtures and the fixed
// real_recipient_r0 payout address below.
#[test]
fn claim_deducts_fee_and_sends_to_fee_recipient() {
    // 500 bps = 5% of a 5 * 100 = 500 stroop pot → 25 fee, 475 net.
    // Asserts the `apply_fee` invariant fee + net == payout on-chain.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).with_fee(500).build(&env);
    let fee_recipient = s.fee_recipient.clone();
    let client = ContractClient::new(&s.env, &s.client_id);
    let token_client = token::Client::new(&s.env, &s.token);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let payout = s.contribution * (s.size as i128);
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

    let fee = 25i128;
    let net = payout - fee;
    assert_eq!(fee + net, payout, "apply_fee must preserve the amount");
    assert_eq!(token_client.balance(&fee_recipient), fee);
    assert_eq!(token_client.balance(&recipient), net);
    assert_eq!(token_client.balance(&s.client_id), 0);
}

#[test]
fn claim_skips_fee_transfer_when_fee_bps_zero() {
    // fee_bps = 0 must behave exactly as a pre-fee circle: one payout
    // transfer to the recipient, nothing to the (ignored) fee recipient.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).with_fee(0).build(&env);
    let fee_recipient = s.fee_recipient.clone();
    let client = ContractClient::new(&s.env, &s.client_id);
    let token_client = token::Client::new(&s.env, &s.token);

    for m in s.members.iter() {
        client.fund(&s.circle_id, m);
    }

    let payout = s.contribution * (s.size as i128);
    let recipient = real_recipient_r0(&s.env);
    client.claim(
        &s.circle_id,
        &recipient,
        &real_nullifier_hash(&s.env),
        &real_external_nullifier_round0(&s.env),
        &real_valid_proof(&s.env),
    );

    assert_eq!(token_client.balance(&fee_recipient), 0);
    assert_eq!(token_client.balance(&recipient), payout);
}

#[test]
fn fee_is_immutable_after_creation() {
    // There is deliberately no setter for fee_bps/fee_recipient (ADR 003):
    // once committed at create_circle, every public entrypoint leaves them
    // exactly as they were. Funding and claiming both write the circle on
    // every call; asserting the fee survives fund (and the earlier
    // create_circle_accepts_maximum_fee_bps / claim tests) pins that down.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).with_fee(250).build(&env);
    let fee_recipient = s.fee_recipient.clone();
    let client = ContractClient::new(&s.env, &s.client_id);

    let circle_before = client.get_circle(&s.circle_id);
    assert_eq!(circle_before.fee_bps, 250);
    assert_eq!(circle_before.fee_recipient, fee_recipient);

    client.fund(&s.circle_id, &s.members[0]);

    let circle_after = client.get_circle(&s.circle_id);
    assert_eq!(circle_after.fee_bps, 250);
    assert_eq!(circle_after.fee_recipient, fee_recipient);
    assert_eq!(circle_after.pot, s.contribution);
}

// ---- Issue #252: apply_fee helper ----

#[test]
fn apply_fee_zero_bps_yields_no_fee() {
    let env = Env::default();
    assert_eq!(apply_fee(&env, 0, 12_345), (0, 12_345));
}

#[test]
fn apply_fee_full_bps_takes_entire_amount() {
    let env = Env::default();
    assert_eq!(apply_fee(&env, 10_000, 12_345), (12_345, 0));
}

#[test]
fn apply_fee_truncates_toward_zero() {
    let env = Env::default();
    // 500 bps = 5%: 12_345 * 500 / 10_000 = 617 (truncated), net 11_728.
    assert_eq!(apply_fee(&env, 500, 12_345), (617, 11_728));
    assert_eq!(617 + 11_728, 12_345);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // InvalidFeeParams
fn apply_fee_rejects_out_of_range_bps() {
    let env = Env::default();
    apply_fee(&env, 10_001, 100);
}
