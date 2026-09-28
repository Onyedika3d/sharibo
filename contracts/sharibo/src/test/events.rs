#![cfg(test)]

use super::*;

// ---- Contract event tests ----
//
// Each state-changing entrypoint emits a contract event so off-chain
// observers can react without polling. These tests pin the topic tuple and
// the data payload of each event.

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
    let circle_id = client.create_circle(&admin, &token, &root, &contribution, &size, &0u32, &vk, &0u32, &Address::generate(&env));

    let env_ref = env.clone();
    let events = env.events().all();
    let event = events
        .iter()
        .find(|(_, topics, _)| {
            let t0: Option<Symbol> = topics.get(0).and_then(|v| v.try_into_val(&env_ref).ok());
            let t1: Option<Symbol> = topics.get(1).and_then(|v| v.try_into_val(&env_ref).ok());
            t0 == Some(symbol_short!("circle")) && t1 == Some(symbol_short!("created"))
        })
        .unwrap();

    let (_, topics, data) = event;
    let t0: Symbol = topics.get(0).unwrap().try_into_val(&env_ref).unwrap();
    assert_eq!(t0, symbol_short!("circle"));
    let topic2: u64 = topics.get(2).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic2, circle_id);

    let (event_admin, event_token, event_contribution, event_size): (Address, Address, i128, u32) =
        data.try_into_val(&env).unwrap();
    assert_eq!(event_admin, admin);
    assert_eq!(event_token, token);
    assert_eq!(event_contribution, contribution);
    assert_eq!(event_size, size);
}

#[test]
fn fund_emits_funded_event() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let from = s.members[0].clone();
    client.fund(&s.circle_id, &from);

    let env_ref = s.env.clone();
    let events = s.env.events().all();
    let event = events
        .iter()
        .find(|(_, topics, _)| {
            let t0: Option<Symbol> = topics.get(0).and_then(|v| v.try_into_val(&env_ref).ok());
            let t1: Option<Symbol> = topics.get(1).and_then(|v| v.try_into_val(&env_ref).ok());
            t0 == Some(symbol_short!("circle")) && t1 == Some(symbol_short!("funded"))
        })
        .unwrap();

    let (_, topics, data) = event;
    let t0: Symbol = topics.get(0).unwrap().try_into_val(&env_ref).unwrap();
    assert_eq!(t0, symbol_short!("circle"));
    let topic2: u64 = topics.get(2).unwrap().try_into_val(&s.env).unwrap();
    assert_eq!(topic2, s.circle_id);

    let (event_from, new_pot, target): (Address, i128, i128) =
        data.try_into_val(&s.env).unwrap();
    assert_eq!(event_from, from);
    assert_eq!(new_pot, s.contribution);
    assert_eq!(target, s.contribution * (s.size as i128));
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

    let env_ref = s.env.clone();
    let events = s.env.events().all();
    let event = events
        .iter()
        .find(|(_, topics, _)| {
            let t0: Option<Symbol> = topics.get(0).and_then(|v| v.try_into_val(&env_ref).ok());
            let t1: Option<Symbol> = topics.get(1).and_then(|v| v.try_into_val(&env_ref).ok());
            t0 == Some(symbol_short!("circle")) && t1 == Some(symbol_short!("claimed"))
        })
        .unwrap();

    let (_, topics, data) = event;
    let t0: Symbol = topics.get(0).unwrap().try_into_val(&env_ref).unwrap();
    assert_eq!(t0, symbol_short!("circle"));
    let topic2: u64 = topics.get(2).unwrap().try_into_val(&s.env).unwrap();
    assert_eq!(topic2, s.circle_id);

    let (round, amount, event_recipient): (u32, i128, Address) =
        data.try_into_val(&s.env).unwrap();
    assert_eq!(round, 0);
    assert_eq!(amount, s.contribution * (s.size as i128));
    assert_eq!(event_recipient, recipient);
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

    let env_ref = s.env.clone();
    let events = s.env.events().all();
    let event = events
        .iter()
        .find(|(_, topics, _)| {
            let t0: Option<Symbol> = topics.get(0).and_then(|v| v.try_into_val(&env_ref).ok());
            let t1: Option<Symbol> = topics.get(1).and_then(|v| v.try_into_val(&env_ref).ok());
            t0 == Some(symbol_short!("circle")) && t1 == Some(symbol_short!("cancelled"))
        })
        .unwrap();

    let (_, topics, data) = event;
    let t0: Symbol = topics.get(0).unwrap().try_into_val(&env_ref).unwrap();
    assert_eq!(t0, symbol_short!("circle"));
    let topic2: u64 = topics.get(2).unwrap().try_into_val(&s.env).unwrap();
    assert_eq!(topic2, s.circle_id);

    let (refunded_count, refunded_total): (u32, i128) =
        data.try_into_val(&s.env).unwrap();
    assert_eq!(refunded_count, 2);
    assert_eq!(refunded_total, s.contribution * 2i128);
}
