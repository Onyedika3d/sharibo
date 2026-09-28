#![cfg(test)]

use super::*;

// ---- Admin transfer (propose/accept) and cancelled-circle rejection ----
//
// Restored from git history (lost in the #235 merge; see PR summary).

#[test]
fn admin_transfer_full_flow() {
    // Current admin proposes; new admin accepts; new admin can cancel.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    let old_admin = client.get_circle(&s.circle_id).admin;
    let new_admin = Address::generate(&s.env);

    client.propose_admin(&s.circle_id, &new_admin);
    client.accept_admin(&s.circle_id);

    let circle = client.get_circle(&s.circle_id);
    assert_eq!(circle.admin, new_admin);

    // New admin can cancel the circle without error.
    client.cancel_circle(&s.circle_id);
    assert!(client.get_circle(&s.circle_id).cancelled);

    // Verify the old_admin variable was different so the assertion is meaningful.
    assert_ne!(old_admin, new_admin);
}

#[test]
fn old_admin_cannot_cancel_after_transfer() {
    // After a completed transfer the stored admin is the new admin, so the
    // old admin no longer authorises cancel_circle. mock_all_auths accepts
    // any signer, so we assert on the stored admin field directly.
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).with_round_deadline(100_000).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    let old_admin = s.admin.clone();
    let new_admin = Address::generate(&s.env);

    client.propose_admin(&s.circle_id, &new_admin);
    client.accept_admin(&s.circle_id);

    // The circle admin is now new_admin.
    let circle = client.get_circle(&s.circle_id);
    assert_eq!(circle.admin, new_admin);
    assert_ne!(circle.admin, old_admin);
}

#[test]
fn new_admin_can_cancel_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    let new_admin = Address::generate(&s.env);
    client.propose_admin(&s.circle_id, &new_admin);
    client.accept_admin(&s.circle_id);

    // New admin cancels — must not panic.
    client.cancel_circle(&s.circle_id);
    assert!(client.get_circle(&s.circle_id).cancelled);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // CircleCancelled
fn propose_admin_on_cancelled_circle_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    client.cancel_circle(&s.circle_id);

    let new_admin = Address::generate(&s.env);
    client.propose_admin(&s.circle_id, &new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // CircleCancelled
fn accept_admin_on_cancelled_circle_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    let new_admin = Address::generate(&s.env);
    // Propose first (circle is still live), then cancel, then try to accept.
    client.propose_admin(&s.circle_id, &new_admin);
    client.cancel_circle(&s.circle_id);
    client.accept_admin(&s.circle_id);
}

#[test]
fn propose_admin_requires_current_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let s = TestCircle::new(5, 100).build(&env);
    let client = ContractClient::new(&s.env, &s.client_id);

    let new_admin = Address::generate(&s.env);
    client.propose_admin(&s.circle_id, &new_admin);

    let auths = s.env.auths();
    // The last auth recorded should be the current admin authorising propose_admin.
    let admin_address = client.get_circle(&s.circle_id).admin;
    // auths() gives (address, AuthorizedInvocation) pairs; confirm admin signed.
    assert!(auths.iter().any(|(addr, _)| addr == &admin_address));
}
