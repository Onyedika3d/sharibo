#![cfg(test)]

use super::*;

// ---- Issue #82: admin cancel/refund path ----

#[test]
fn cancel_refunds_partial_funders_and_closes_circle() {
    // Scenario: 4 of 5 members fund, the 5th never shows up.
    // Admin cancels; all 4 existing funders are refunded exactly
    // `contribution` each, and the circle is permanently closed.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let _token_admin_client = token::StellarAssetClient::new(&s.env, &s.token);
    let token_client = token::Client::new(&s.env, &s.token);

    // Mint enough for 4 funders (setup only mints `contribution` per member).
    let funders: StdVec<Address> = s.members.iter().take(4).cloned().collect();
    for f in funders.iter() {
        client.fund(&s.circle_id, f);
    }

    let circle_before = client.get_circle(&s.circle_id);
    assert_eq!(circle_before.pot, s.contribution * 4);
    assert_eq!(circle_before.contributors.len(), 4);

    // Record balances before cancel.
    let before: StdVec<i128> = funders.iter().map(|f| token_client.balance(f)).collect();

    let _admin = client.get_circle(&s.circle_id).admin;
    client.cancel_circle(&s.circle_id);

    // Every funder must have been refunded exactly their contribution.
    for (f, bal_before) in funders.iter().zip(before.iter()) {
        assert_eq!(
            token_client.balance(f),
            bal_before + s.contribution,
            "funder {f:?} not fully refunded"
        );
    }

    let circle_after = client.get_circle(&s.circle_id);
    assert_eq!(circle_after.pot, 0);
    assert!(circle_after.cancelled);
    assert_eq!(circle_after.contributors.len(), 0);

    // Contract holds no tokens.
    assert_eq!(token_client.balance(&s.client_id), 0);
}

// ---- Issue #318: cancel before any contributor has funded ----

#[test]
fn cancel_zero_contributors_is_clean_close() {
    // Cancel immediately after create_circle, before anyone funds.
    // The contributors Vec is empty, so the refund loop must be a no-op.
    // Expected outcome: cancelled == true, pot == 0, no token movement.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    let token_client = token::Client::new(&s.env, &s.token);

    // Sanity: nothing in the pot yet.
    let circle_before = client.get_circle(&s.circle_id);
    assert_eq!(circle_before.pot, 0);
    assert_eq!(circle_before.contributors.len(), 0);
    assert!(!circle_before.cancelled);

    // Contract holds no tokens at this point.
    let contract_balance_before = token_client.balance(&s.client_id);
    assert_eq!(contract_balance_before, 0);

    client.cancel_circle(&s.circle_id);

    let circle_after = client.get_circle(&s.circle_id);
    assert_eq!(circle_after.pot, 0, "pot must remain 0 after cancelling an empty circle");
    assert!(circle_after.cancelled, "circle must be marked cancelled");
    assert_eq!(circle_after.contributors.len(), 0, "contributors vec must stay empty");

    // No tokens moved: contract balance is still 0.
    assert_eq!(
        token_client.balance(&s.client_id),
        0,
        "contract token balance must not change"
    );

    // No member token balance should have changed either.
    for m in s.members.iter() {
        assert_eq!(
            token_client.balance(m),
            s.contribution,
            "member {m:?} balance must be unchanged — no refund should have fired"
        );
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // CircleCancelled
fn double_cancel_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);
    client.cancel_circle(&s.circle_id);
    client.cancel_circle(&s.circle_id);
}
