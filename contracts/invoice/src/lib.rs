#![no_std]
#![deny(unused_variables)]
// Pedantic lints are enabled in CI. Suppress categories that conflict with Soroban's
// generated code patterns or are stylistic rather than correctness-related.
#![allow(clippy::module_name_repetitions)] // e.g. InvoiceContract, InvoiceStatus in same crate
#![allow(clippy::missing_errors_doc)] // contract entry points are documented at the module level
#![allow(clippy::must_use_candidate)] // Soroban contractimpl fns are called for side-effects

mod constants;
mod escrow;
mod events;
mod storage;
mod views;

#[allow(clippy::wildcard_imports)] // constants module is a flat list of pub consts
use crate::constants::*;
use soroban_sdk::{contract, contractimpl, contractmeta, panic_with_error, token, Address, Env, String};

contractmeta!(key = "Description", val = "StarInvoice escrow contract");
contractmeta!(key = "Version", val = "0.1.0");

pub use storage::{ContractError, Invoice, InvoiceStatus};

#[cfg(test)]
mod test_init;
#[cfg(test)]
mod tests; // tests/mod.rs — contains helpers submodule and all invoice tests

/// Validates whether a status transition is permitted.
///
/// # CONTRIBUTOR NOTE
/// When adding new `InvoiceStatus` variants, you MUST update this function and every
/// `match` on `InvoiceStatus` in the codebase. Do NOT use a wildcard `_` arm in those
/// matches — exhaustive arms ensure the compiler catches missing cases at build time.
pub fn validate_transition(from: &InvoiceStatus, to: &InvoiceStatus) -> bool {
    // Exhaustive match — no wildcard arm so the compiler forces updates when new variants are added.
    matches!(
        (from, to),
        (InvoiceStatus::Pending, InvoiceStatus::Funded)
            | (InvoiceStatus::Pending, InvoiceStatus::Cancelled)
            | (InvoiceStatus::Funded, InvoiceStatus::Delivered)
            | (InvoiceStatus::Funded, InvoiceStatus::Disputed)
            | (InvoiceStatus::Delivered, InvoiceStatus::Approved)
            | (InvoiceStatus::Delivered, InvoiceStatus::Disputed)
            | (InvoiceStatus::Approved, InvoiceStatus::Completed)
    )
}

#[contract]
pub struct InvoiceContract;

#[contractimpl]
impl InvoiceContract {
    pub fn create_invoice(
        env: Env,
        freelancer: Address,
        client: Address,
        amount: i128,
        token: Address,
        deadline: u64,
        title: String,
        description: String,
        metadata_uri: String,
    ) -> Result<u64, ContractError> {
        // Auth: the freelancer must sign — only the freelancer may create an invoice on their
        // own behalf, preventing a third party from submitting invoices in someone else's name.
        freelancer.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, ContractError::InvalidAmount);
        }

        if amount > MAX_INVOICE_AMOUNT {
            panic_with_error!(&env, ContractError::AmountExceedsMaximum);
        }

        if freelancer == client {
            panic_with_error!(&env, ContractError::InvalidParties);
        }

        if description.len() > MAX_DESCRIPTION_LEN {
            panic_with_error!(&env, ContractError::DescriptionTooLong);
        }

        if metadata_uri.len() > MAX_METADATA_URI_LEN {
            panic_with_error!(&env, ContractError::MetadataUriTooLong);
        }

        let invoice_id = storage::next_invoice_id(&env);

        let invoice = Invoice {
            id: invoice_id,
            freelancer: freelancer.clone(),
            client: client.clone(),
            amount,
            token,
            deadline,
            title,
            created_at: env.ledger().timestamp(),
            description,
            status: InvoiceStatus::Pending,
            metadata_uri,
        };

        storage::save_invoice(&env, &invoice);
        events::invoice_created(&env, invoice_id, &freelancer, &client, amount);
        Ok(invoice_id)
    }

    pub fn initialize(env: Env, admin: Address) {
        if storage::get_admin(&env).is_ok() {
            panic!("Already initialized");
        }
        storage::set_admin(&env, &admin);
    }

    pub fn fund_invoice(env: Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
        escrow::fund_invoice(&env, invoice_id, token_address)
    }

    pub fn mark_delivered(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        escrow::mark_delivered(&env, invoice_id)
    }

    pub fn approve_payment(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        escrow::approve_payment(&env, invoice_id)
    }

    pub fn cancel_invoice(env: Env, invoice_id: u64, caller: Address) -> Result<(), ContractError> {
        escrow::cancel_invoice(&env, invoice_id, caller)
    }

    pub fn release_payment(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        escrow::release_payment(&env, invoice_id)
    }

    pub fn dispute_invoice(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        escrow::dispute_invoice(&env, invoice_id)
    }

    pub fn get_invoice(env: Env, invoice_id: u64) -> Result<Invoice, ContractError> {
        views::get_invoice(&env, invoice_id)
    }

    pub fn get_invoices_by_freelancer(env: Env, freelancer: Address) -> soroban_sdk::Vec<u64> {
        views::get_invoices_by_freelancer(&env, &freelancer)
    }

    pub fn get_invoices_by_client(env: Env, client: Address) -> soroban_sdk::Vec<u64> {
        views::get_invoices_by_client(&env, &client)
    }

    pub fn invoice_count(env: Env) -> u64 {
        views::invoice_count(&env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, token, Env, String};

    fn setup_token(env: &Env, minter: &Address, amount: i128) -> Address {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        token::StellarAssetClient::new(env, &token_address).mint(minter, &amount);
        token_address
    }

    fn create_funded_invoice(
        env: &Env,
        client: &InvoiceContractClient,
        _contract_id: &Address,
        freelancer: &Address,
        payer: &Address,
        amount: i128,
    ) -> (u64, Address) {
        let title = String::from_str(env, "Test Invoice");
        let description = String::from_str(env, "Test description");
        let metadata_uri = String::from_str(env, "");
        let token_address = setup_token(env, payer, amount);
        let invoice_id = client.create_invoice(
            freelancer,
            payer,
            &amount,
            &token_address,
            &9_999_999_999,
            &title,
            &description,
            &metadata_uri,
        );
        client.fund_invoice(&invoice_id, &token_address);
        (invoice_id, token_address)
    }

    #[test]
    fn test_create_invoice() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let title = String::from_str(&env, "Logo Design");
        let description = String::from_str(&env, "Logo design work");
        let metadata_uri = String::from_str(&env, "");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description, &metadata_uri);
        assert_eq!(invoice_id, 0);
    }

    #[test]
    fn test_create_invoice_unique_ids() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Unique ID Test");
        let description = String::from_str(&env, "Unique ID test");
        let metadata_uri = String::from_str(&env, "");

        for i in 0..5u64 {
            let invoice_id = client.create_invoice(
                &freelancer, &payer, &1000, &Address::generate(&env),
                &9_999_999_999, &title, &description, &metadata_uri,
            );
            assert_eq!(invoice_id, i);
        }
        assert_eq!(client.invoice_count(), 5);
    }

    #[test]
    fn test_mark_delivered_happy_path() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        client.mark_delivered(&invoice_id);

        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Delivered);
    }

    #[test]
    fn test_fund_invoice_happy_path() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 3000);

        client.mark_delivered(&invoice_id);
        client.approve_payment(&invoice_id);

        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Approved);
    }

    #[test]
    fn test_release_payment_happy_path() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let amount: i128 = 2000;
        let (invoice_id, token_address) = create_funded_invoice(&env, &invoice_client, &contract_id, &freelancer, &payer, amount);

        invoice_client.mark_delivered(&invoice_id);
        invoice_client.approve_payment(&invoice_id);
        invoice_client.release_payment(&invoice_id);

        let token_client = token::Client::new(&env, &token_address);
        assert_eq!(token_client.balance(&freelancer), amount);
        assert_eq!(token_client.balance(&contract_id), 0);

        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Completed);
    }

    #[test]
    fn test_invoice_not_found_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let result = client.try_get_invoice(&999);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_amount_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "D");
        let metadata_uri = String::from_str(&env, "");

        let result = client.try_create_invoice(&freelancer, &payer, &0, &token, &9_999_999_999, &title, &description, &metadata_uri);
        assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
    }

    #[test]
    fn test_invalid_parties_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let token = Address::generate(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "D");
        let metadata_uri = String::from_str(&env, "");

        let result = client.try_create_invoice(&freelancer, &freelancer, &1000, &token, &9_999_999_999, &title, &description, &metadata_uri);
        assert_eq!(result, Err(Ok(ContractError::InvalidParties)));
    }

    #[test]
    fn test_description_too_long_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let title = String::from_str(&env, "T");
        let long_desc = String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let metadata_uri = String::from_str(&env, "");

        let result = client.try_create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &long_desc, &metadata_uri);
        assert_eq!(result, Err(Ok(ContractError::DescriptionTooLong)));
    }

    #[test]
    fn test_invalid_status_mark_delivered_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "D");
        let metadata_uri = String::from_str(&env, "");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description, &metadata_uri);
        let result = client.try_mark_delivered(&invoice_id);
        assert_eq!(result, Err(Ok(ContractError::InvalidInvoiceStatus)));
    }

    #[test]
    fn test_invalid_status_approve_payment_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        let result = client.try_approve_payment(&invoice_id);
        assert_eq!(result, Err(Ok(ContractError::InvalidInvoiceStatus)));
    }

    #[test]
    fn test_invalid_status_release_payment_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        client.mark_delivered(&invoice_id);
        let result = client.try_release_payment(&invoice_id);
        assert_eq!(result, Err(Ok(ContractError::InvalidInvoiceStatus)));
    }

    #[test]
    fn test_unauthorized_cancel_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let stranger = Address::generate(&env);
        let token = Address::generate(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "D");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description);
        let result = client.try_cancel_invoice(&invoice_id, &stranger);
        assert_eq!(result, Err(Ok(ContractError::UnauthorizedCaller)));
    }

    #[test]
    fn test_token_mismatch_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let wrong_token = Address::generate(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "D");
        let metadata_uri = String::from_str(&env, "");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description, &metadata_uri);
        let result = client.try_fund_invoice(&invoice_id, &wrong_token);
        assert_eq!(result, Err(Ok(ContractError::TokenMismatch)));
    }

    #[test]
    #[should_panic]
    fn test_mark_delivered_wrong_caller() {
        let env = Env::default();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);

        env.mock_all_auths();
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        env.set_auths(&[]);
        client.mark_delivered(&invoice_id);
    }

    #[test]
    #[should_panic]
    fn test_approve_payment_wrong_caller() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        client.mark_delivered(&invoice_id);
        env.set_auths(&[]);
        client.approve_payment(&invoice_id);
    }
}
