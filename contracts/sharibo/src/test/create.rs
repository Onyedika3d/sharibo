#![cfg(test)]

use super::*;

#[test]
fn create_circle_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);

    let root = real_root(&env);
    let vk = real_verification_key(&env);
    client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &5u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // InvalidFeeParams
fn create_circle_rejects_fee_bps_out_of_range() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let root = real_root(&env);
    let vk = real_verification_key(&env);
    let fee_recipient = Address::generate(&env);
    client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &5u32,
        &0u32,
        &vk,
        &10_001u32,
        &fee_recipient,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")] // InvalidRecipient
fn create_circle_rejects_contract_as_fee_recipient() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let root = real_root(&env);
    let vk = real_verification_key(&env);
    client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &5u32,
        &0u32,
        &vk,
        &500u32,
        &contract_id,
    );
}

#[test]
fn create_circle_accepts_maximum_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).with_fee(10_000).build(&env);
    let fee_recipient = s.fee_recipient.clone();
    let circle = ContractClient::new(&s.env, &s.client_id).get_circle(&s.circle_id);
    assert_eq!(circle.fee_bps, 10_000);
    assert_eq!(circle.fee_recipient, fee_recipient);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn create_circle_rejects_size_above_max_capacity() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let root = real_root(&env);
    let vk = real_verification_key(&env);

    // The Merkle tree holds at most 2^4 = 16 commitments (circuits/config.json);
    // a larger size can never be fully claimed.
    let oversized = MAX_CIRCLE_SIZE + 1;
    client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &oversized,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}

#[test]
fn create_circle_accepts_max_capacity_size() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let root = real_root(&env);
    let vk = real_verification_key(&env);

    let circle_id = client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &MAX_CIRCLE_SIZE,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.size, MAX_CIRCLE_SIZE);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn create_circle_rejects_pot_target_overflow() {
    // contribution * size overflows i128, so create_circle rejects the
    // circle at creation time (checked pot-target arithmetic,
    // InvalidCircleParams) before any funds move.
    let env = Env::default();
    env.mock_all_auths();
    let _ = TestCircle::new(2, i128::MAX).build(&env);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn create_circle_rejects_truncated_ic() {
    // create_circle validates vk shape up front: the circuit exposes
    // [nullifierHash, root, externalNullifier, recipientHash], so ic must
    // hold one point per public signal plus one (5 total). A truncated ic
    // is rejected at creation (InvalidCircleParams) before any circle or
    // funds exist. verify_groth16 keeps its own length guard as
    // defense-in-depth against a hand-crafted vk ever reaching claim.
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);

    let mut truncated_vk = real_verification_key(&env);
    assert_eq!(truncated_vk.ic.len(), 5);
    truncated_vk.ic.pop_back(); // Remove the last ic point; len is now 4.
    assert_eq!(truncated_vk.ic.len(), 4);

    client.create_circle(
        &admin,
        &token,
        &real_root(&env),
        &100i128,
        &5u32,
        &0u32,
        &truncated_vk,
        &0u32,
        &Address::generate(&env),
    );
    unreachable!("create_circle with a truncated vk must revert");
}

// ---- Restored validation cases (lost in the #235 merge, recovered from
// git history; see PR summary) ----

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn create_circle_rejects_zero_size() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let vk = real_verification_key(&env);

    client.create_circle(
        &admin,
        &token,
        &real_root(&env),
        &100i128,
        &0u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn create_circle_rejects_wrong_vk_length() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let mut vk = real_verification_key(&env);
    vk.ic.pop_back();

    client.create_circle(
        &admin,
        &token,
        &real_root(&env),
        &100i128,
        &5u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn create_circle_rejects_creation_time_overflow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let vk = real_verification_key(&env);

    client.create_circle(
        &admin,
        &token,
        &real_root(&env),
        &i128::MAX,
        &2u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn zero_size_circle_is_rejected_at_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);

    let root = real_root(&env);
    let vk = real_verification_key(&env);

    // Without the guard, `size = 0` makes `pot_target = contribution * size = 0`.
    // The first claim sees `pot (0) == target (0)`, passes the `RoundNotFunded`
    // check, and then burns a nullifier while advancing an otherwise empty pot.
    client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &0u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn zero_contribution_circle_is_rejected_at_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);

    let root = real_root(&env);
    let vk = real_verification_key(&env);

    // Without validation, `contribution = 0` creates a circle whose `pot_target`
    // is also 0. From there the same empty-pot claim regression is reachable.
    client.create_circle(
        &admin,
        &token,
        &root,
        &0i128,
        &5u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidCircleParams
fn negative_contribution_is_rejected_at_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);

    let root = real_root(&env);
    let vk = real_verification_key(&env);

    // A negative contribution is also invalid: it would make the round target
    // non-positive and let the same empty-pot edge case slip through during claim.
    client.create_circle(
        &admin,
        &token,
        &root,
        &(-100i128),
        &5u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
}
