use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

use crate::{InvoiceContract, InvoiceContractClient};

/// Standard invoice amount used across tests.
pub const TEST_AMOUNT: i128 = 1_000;

/// Default deadline far in the future.
pub const TEST_DEADLINE: u64 = 9_999_999_999;

/// Creates a default test environment with all auths mocked and a registered contract.
/// Returns `(env, contract_id, client)`.
///
/// The returned `InvoiceContractClient` borrows from `env`, so callers must bind all
/// three values and keep `env` alive for the duration of the test.
pub fn setup_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, InvoiceContract);
    (env, contract_id)
}

/// Registers a Stellar asset contract, mints `amount` tokens to `minter`, and returns the token address.
pub fn setup_token(env: &Env, minter: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    token::StellarAssetClient::new(env, &token_address).mint(minter, &amount);
    token_address
}

/// Creates an invoice and funds it, returning `(invoice_id, token_address)`.
pub fn create_test_invoice(
    env: &Env,
    client: &InvoiceContractClient,
    freelancer: &Address,
    payer: &Address,
    amount: i128,
) -> (u64, Address) {
    let token_address = setup_token(env, payer, amount);
    let invoice_id = client.create_invoice(
        freelancer,
        payer,
        &amount,
        &token_address,
        &TEST_DEADLINE,
        &String::from_str(env, "Test Invoice"),
        &String::from_str(env, "Test description"),
    );
    client.fund_invoice(&invoice_id, &token_address);
    (invoice_id, token_address)
}
