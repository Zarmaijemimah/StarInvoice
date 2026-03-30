use soroban_sdk::{testutils::Address as _, Address};

use super::tests::helpers::setup_env;
use super::*;

#[test]
fn test_initialize_sets_admin() {
    let (env, contract_id) = setup_env();
    let client = InvoiceContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let stored_admin = env.as_contract(&contract_id, || storage::get_admin(&env).unwrap());
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_initialize_twice_panics() {
    let (env, contract_id) = setup_env();
    let client = InvoiceContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.initialize(&Address::generate(&env));
}
