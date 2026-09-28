#![cfg(test)]

use super::*;
use std::collections::BTreeMap;

// ---- Proptest: apply_fee invariants ----
//
// Two sub-suites:
//
// 1. Valid domain — lossless split: fee + net == amount exactly.
//    Integer truncation in `fee = fee_bps * amount / 10_000` rounds *down*;
//    the remainder always lands entirely in `net` — no tokens created or lost.
//    Also asserts fee <= amount (net is non-negative) for the valid range.
//
//    Domain:
//      amount  : 0 ..= i128::MAX / 2   (avoids intermediate multiplication
//                 overflow, since fee_bps ≤ 10_000 and
//                 10_000 * (i128::MAX / 2) < i128::MAX)
//      fee_bps : 0 ..= 10_000          (0% – 100%)
//
// 2. Out-of-range rejection — fee_bps > 10_000 or amount < 0 must panic
//    with Error::InvalidFeeParams rather than silently produce a negative net.
mod proptest_apply_fee {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fee_plus_net_equals_amount(
            amount  in 0_i128..=(i128::MAX / 2),
            fee_bps in 0_u32..=10_000_u32,
        ) {
            let env = Env::default();
            let (fee, net) = apply_fee(&env, fee_bps, amount);
            prop_assert_eq!(
                fee + net,
                amount,
                "apply_fee({}, {}) = ({}, {}); fee + net = {}",
                fee_bps, amount, fee, net, fee + net
            );
            // net must never be negative — fee cannot exceed the amount.
            prop_assert!(net >= 0, "apply_fee({fee_bps}, {amount}) produced negative net {net}");
        }

        #[test]
        fn rejects_fee_bps_above_10000(
            amount  in 0_i128..=(i128::MAX / 2),
            excess  in 1_u32..=u32::MAX - 10_000,
        ) {
            let env = Env::default();
            let fee_bps = 10_000_u32 + excess;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| apply_fee(&env, fee_bps, amount)));
            prop_assert!(result.is_err(), "apply_fee({fee_bps}, {amount}) should have panicked");
        }

        #[test]
        fn rejects_negative_amount(
            amount  in i128::MIN..=-1_i128,
            fee_bps in 0_u32..=10_000_u32,
        ) {
            let env = Env::default();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| apply_fee(&env, fee_bps, amount)));
            prop_assert!(result.is_err(), "apply_fee({fee_bps}, {amount}) should have panicked");
        }
    }
}

// ---- Proptest: circle invariants (random legal sequences of fund/claim/cancel)
//
// Restored from git history (lost in the #235 merge; see PR summary), adapted
// to the current contract:
// - the committed fixtures prove exactly one claim per circle (round 0), so
//   the claim action is gated on `round == 0` — a second claim would reuse
//   the nullifier and revert with AlreadyClaimed;
// - the claim pays the registered fixture recipient (issue #266), not a
//   random address;
// - each fund re-mints the funder first: members hold only one contribution
//   at a time, so without re-minting a second fund from the same member
//   would fail on insufficient balance.
mod proptest_circle_invariants {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn random_legal_sequence(actions in proptest::collection::vec(0u8..=2u8, 1..20)) {
            let env = Env::default();
            env.mock_all_auths();
            let s = TestCircle::new(5, 100).build(&env);
            let client = ContractClient::new(&s.env, &s.client_id);
            let contract_id = s.client_id.clone();
            let circle_id = s.circle_id;
            let funders = s.members.clone();
            let token_admin_client = token::StellarAssetClient::new(&s.env, &s.token);

            for a in actions.into_iter() {
                match a {
                    0 => {
                        // fund a member if the round is open and not cancelled
                        let circle = client.get_circle(&circle_id);
                        if circle.pot < s.contribution * (s.size as i128) && !circle.cancelled {
                            let ix = (env.crypto().sha256(&Bytes::from_array(&env, &[a])).to_array()[0] as usize) % (funders.len());
                            let who = funders.get(ix).unwrap();
                            token_admin_client.mint(who, &s.contribution);
                            client.fund(&circle_id, who);
                            assert_invariants(&env, &contract_id, &client, circle_id);
                        }
                    }
                    1 => {
                        // claim only when full, and only in round 0 (the
                        // fixtures prove a single claim per circle)
                        let circle = client.get_circle(&circle_id);
                        if circle.pot == circle.contribution * (circle.size as i128)
                            && !circle.cancelled
                            && circle.round == 0
                        {
                            let recipient = real_recipient_r0(&env);
                            let nullifier_hash = real_nullifier_hash(&env);
                            let external_nullifier = expected_external_nullifier(&env, circle_id, circle.round);
                            let proof = real_valid_proof(&env);
                            client.claim(&circle_id, &recipient, &nullifier_hash, &external_nullifier, &proof);
                            assert_invariants(&env, &contract_id, &client, circle_id);
                        }
                    }
                    2 => {
                        // cancel if not already cancelled
                        let circle = client.get_circle(&circle_id);
                        if !circle.cancelled {
                            client.cancel_circle(&circle_id);
                            assert_invariants(&env, &contract_id, &client, circle_id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---- Config invariant: MAX_CIRCLE_SIZE matches the circuit ----

#[test]
fn max_circle_size_matches_circuit_levels() {
    // The bound is asserted against the source of truth, not just commented:
    // bumping `levels` in circuits/config.json without updating MAX_CIRCLE_SIZE
    // fails this test, forcing a deliberate review of the contract constant
    // (and a redeploy, since the bound is compiled into the WASM).
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../circuits/config.json");
    let contents = std::fs::read_to_string(&config_path)
        .expect("circuits/config.json not found; run tests from a full checkout");

    let levels = parse_config_levels(&contents)
        .unwrap_or_else(|| panic!("circuits/config.json must contain a numeric \"levels\" field: {contents}"));

    // 2^levels computed in u64 so an absurdly deep circuit still yields a
    // clean assertion failure instead of an integer-overflow panic.
    let capacity = 1u64 << levels;
    assert_eq!(
        MAX_CIRCLE_SIZE as u64,
        capacity,
        "MAX_CIRCLE_SIZE must equal 2^levels ({capacity}) from circuits/config.json \
         — update the constant (and redeploy the contract) when the circuit depth changes",
    );
}

/// Extract the numeric `levels` value from the JSON file contents.
/// The config is `{ "levels": 4 }`; parsed by hand to keep tests dependency-free.
fn parse_config_levels(contents: &str) -> Option<u32> {
    let needle = "\"levels\"";
    let after_key = &contents[contents.find(needle)? + needle.len()..];
    let after_colon = &after_key[after_key.find(':')? + 1..];
    let after_ws = after_colon.trim_start();
    let digits: std::string::String = after_ws
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

// Shared invariant checker for Issue #265
fn assert_invariants(env: &Env, contract_id: &Address, client: &ContractClient, circle_id: u64) {
    // Circle structural invariants
    let circle = client.get_circle(&circle_id);

    // pot == contribution * contributors.len()
    let contrib_count = circle.contributors.len() as i128;
    assert_eq!(circle.pot, circle.contribution * contrib_count, "pot != contribution * contributors.len(): pot={} contribution={} count={}", circle.pot, circle.contribution, contrib_count);

    // pot <= contribution * size
    let target = circle.contribution.checked_mul(circle.size as i128).unwrap_or(i128::MAX);
    assert!(circle.pot <= target, "pot {} > target {}", circle.pot, target);

    // After a successful claim the pot must be zero and contributors cleared
    if circle.round > 0 && circle.pot == 0 && !circle.cancelled {
        assert_eq!(circle.contributors.len(), 0, "after claim contributors must be empty");
    }

    // After cancel: pot == 0, cancelled == true, contributors empty
    if circle.cancelled {
        assert_eq!(circle.pot, 0, "cancelled circle must have pot==0");
        assert!(circle.cancelled, "cancelled flag must be true");
        assert_eq!(circle.contributors.len(), 0, "cancelled circle must have no contributors");
    }

    // The contract's token balance must be at least the sum of all live pots
    // across circles that use the same token. We gather per-token totals and
    // compare with on-chain balances for each token seen.
    let mut totals: BTreeMap<Address, i128> = BTreeMap::new();
    let circle_count = client.get_circle_count();
    for id in 0..circle_count {
        let c = client.get_circle(&id);
        if !c.cancelled {
            let entry = totals.entry(c.token.clone()).or_insert(0i128);
            *entry = entry.checked_add(c.pot).unwrap_or(i128::MAX);
        }
    }

    for (token_addr, total_pots) in totals.iter() {
        let token_client = token::Client::new(env, token_addr);
        let contract_balance = token_client.balance(contract_id);
        assert!(contract_balance >= *total_pots, "contract token balance {} for token {:?} is less than total live pots {}", contract_balance, token_addr, total_pots);
    }
}
