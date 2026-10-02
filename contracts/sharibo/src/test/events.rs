#![cfg(test)]

use super::*;
use crate::events::{
    AdminAccepted, AdminProposed, CircleCancelled, CircleClaimed, CircleCreated, CircleFunded,
    RoundExpired,
};
use soroban_sdk::testutils::Events as _;
use soroban_sdk::Event as _;

#[test]
fn create_circle_emits_created_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let root = real_root(&env);
    let vk = real_verification_key(&env);
    let contribution: i128 = 100;
    let size: u32 = 5;
    let circle_id = client.create_circle(
        &admin,
        &token,
        &root,
        &contribution,
        &size,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );

    let expected = CircleCreated {
        circle_id,
        admin: admin.clone(),
        token: token.clone(),
        contrib: contribution,
        size,
    }
    .to_xdr(&env, &contract_id);
    let all = env.events().all().filter_by_contract(&contract_id);
    assert!(
        all.events().contains(&expected),
        "CircleCreated event not found"
    );
}

#[test]
fn fund_emits_funded_event() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let from = s.members[0].clone();
    client.fund(&s.circle_id, &from);

    let expected = CircleFunded {
        circle_id: s.circle_id,
        from: from.clone(),
        pot: s.contribution,
        target: s.contribution * (s.size as i128),
    }
    .to_xdr(&s.env, &s.client_id);
    let all = s.env.events().all().filter_by_contract(&s.client_id);
    assert!(
        all.events().contains(&expected),
        "CircleFunded event not found"
    );
}

#[test]
fn claim_emits_claimed_event() {
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

    let expected = CircleClaimed {
        circle_id: s.circle_id,
        cround: 0,
        payout: s.contribution * (s.size as i128),
        recipient: recipient.clone(),
    }
    .to_xdr(&s.env, &s.client_id);
    let all = s.env.events().all().filter_by_contract(&s.client_id);
    assert!(
        all.events().contains(&expected),
        "CircleClaimed event not found"
    );
}

#[test]
fn cancel_circle_emits_cancelled_event() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    for m in s.members.iter().take(2) {
        client.fund(&s.circle_id, m);
    }
    client.cancel_circle(&s.circle_id);

    let expected = CircleCancelled {
        circle_id: s.circle_id,
        rcount: 2,
        rtotal: s.contribution * 2i128,
    }
    .to_xdr(&s.env, &s.client_id);
    let all = s.env.events().all().filter_by_contract(&s.client_id);
    assert!(
        all.events().contains(&expected),
        "CircleCancelled event not found"
    );
}

#[test]
fn propose_admin_emits_event() {
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
        &5u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
    let new_admin = Address::generate(&env);
    client.propose_admin(&circle_id, &new_admin);

    let expected = AdminProposed {
        circle_id,
        old_admin: admin.clone(),
        new_admin: new_admin.clone(),
    }
    .to_xdr(&env, &contract_id);
    let all = env.events().all().filter_by_contract(&contract_id);
    assert!(
        all.events().contains(&expected),
        "AdminProposed event not found"
    );
}

#[test]
fn accept_admin_emits_event() {
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
        &5u32,
        &0u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
    let new_admin = Address::generate(&env);
    client.propose_admin(&circle_id, &new_admin);
    client.accept_admin(&circle_id);

    let expected = AdminAccepted {
        circle_id,
        old_admin: admin.clone(),
        new_admin: new_admin.clone(),
    }
    .to_xdr(&env, &contract_id);
    let all = env.events().all().filter_by_contract(&contract_id);
    assert!(
        all.events().contains(&expected),
        "AdminAccepted event not found"
    );
}

#[test]
fn expire_round_emits_event() {
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
        &5u32,
        &10u32,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
    env.ledger().with_mut(|l| {
        l.sequence_number += 20;
    });
    client.expire_round(&circle_id);

    let expected = RoundExpired {
        circle_id,
        eround: 0,
    }
    .to_xdr(&env, &contract_id);
    let all = env.events().all().filter_by_contract(&contract_id);
    assert!(
        all.events().contains(&expected),
        "RoundExpired event not found"
    );
}
