use soroban_sdk::{testutils::Address as _, Address, Env, String};
use crate::{InvoiceContract, InvoiceStatus, ContractError};

#[test]
fn test_admin_initialization_and_update() {
    let env = Env::default();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let contract_id = env.register_contract(None, InvoiceContract);
    let contract_client = crate::InvoiceContractClient::new(&env, &contract_id);

    // Only admin1 can initialize
    admin1.set_initial_balance(&env, 1000);
    contract_client.initialize(&admin1);
    // Only admin1 can set admin
    admin1.require_auth();
    contract_client.set_admin(&admin2);
    // Now only admin2 can set admin
    admin2.require_auth();
    contract_client.set_admin(&admin1);
}

#[test]
fn test_unauthorized_admin_update() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let not_admin = Address::generate(&env);
    let contract_id = env.register_contract(None, InvoiceContract);
    let contract_client = crate::InvoiceContractClient::new(&env, &contract_id);
    contract_client.initialize(&admin);
    // not_admin tries to set admin
    not_admin.require_auth();
    let result = std::panic::catch_unwind(|| contract_client.set_admin(&not_admin));
    assert!(result.is_err());
}

#[test]
fn test_resolve_dispute_by_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let client = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let contract_id = env.register_contract(None, InvoiceContract);
    let contract_client = crate::InvoiceContractClient::new(&env, &contract_id);
    contract_client.initialize(&admin);
    let invoice_id = contract_client.create_invoice(
        &freelancer,
        &client,
        &100,
        &token_address,
        &1000,
        &String::from_str(&env, "Test"),
        &String::from_str(&env, "Desc"),
    ).unwrap();
    // Fund and dispute
    client.require_auth();
    contract_client.fund_invoice(&invoice_id, &token_address).unwrap();
    client.require_auth();
    contract_client.dispute_invoice(&invoice_id).unwrap();
    // Only admin can resolve
    admin.require_auth();
    contract_client.resolve_dispute(&invoice_id, &freelancer).unwrap();
    // Check status
    let invoice = contract_client.get_invoice(&invoice_id).unwrap();
    assert_eq!(invoice.status, InvoiceStatus::Completed);
}

#[test]
fn test_resolve_dispute_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let client = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let contract_id = env.register_contract(None, InvoiceContract);
    let contract_client = crate::InvoiceContractClient::new(&env, &contract_id);
    contract_client.initialize(&admin);
    let invoice_id = contract_client.create_invoice(
        &freelancer,
        &client,
        &100,
        &token_address,
        &1000,
        &String::from_str(&env, "Test"),
        &String::from_str(&env, "Desc"),
    ).unwrap();
    client.require_auth();
    contract_client.fund_invoice(&invoice_id, &token_address).unwrap();
    client.require_auth();
    contract_client.dispute_invoice(&invoice_id).unwrap();
    // Not admin tries to resolve
    freelancer.require_auth();
    let result = std::panic::catch_unwind(|| contract_client.resolve_dispute(&invoice_id, &freelancer));
    assert!(result.is_err());
}

#[test]
fn test_resolve_dispute_invalid_invoice() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, InvoiceContract);
    let contract_client = crate::InvoiceContractClient::new(&env, &contract_id);
    contract_client.initialize(&admin);
    admin.require_auth();
    let result = std::panic::catch_unwind(|| contract_client.resolve_dispute(&9999, &admin));
    assert!(result.is_err());
}
