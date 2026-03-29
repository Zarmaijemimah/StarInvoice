#![no_std]
#![deny(unused_variables)]

mod constants;
mod events;
mod storage;

use crate::constants::*;
use soroban_sdk::{contract, contractimpl, contractmeta, token, Address, Env, String};

contractmeta!(key = "Description", val = "StarInvoice escrow contract");
contractmeta!(key = "Version", val = "0.1.0");

// `Invoice` is returned by the public `get_invoice` contract function, so it must be
// re-exported here for the Soroban-generated client bindings to reference the type.
// `ContractError` and `InvoiceStatus` are likewise part of the public ABI.
// None of these re-exports can be removed without breaking the contract interface.
pub use storage::{ContractError, Invoice, InvoiceStatus};

#[cfg(test)]
mod test_init;

/// Validates whether a status transition is allowed.
///
/// Returns `true` if transitioning `from` → `to` is a legal state change.
///
/// Valid transitions:
/// ```text
/// Pending   → Funded
/// Pending   → Cancelled
/// Funded    → Delivered
/// Funded    → Disputed
/// Delivered → Approved
/// Delivered → Disputed
/// Approved  → Completed
/// ```
pub fn validate_transition(from: &InvoiceStatus, to: &InvoiceStatus) -> bool {
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
    /// Creates a new invoice and stores it on-chain.
    pub fn create_invoice(
        env: Env,
        freelancer: Address,
        client: Address,
        amount: i128,
        token: Address,
        deadline: u64,
        title: String,
        description: String,
    ) -> Result<u64, ContractError> {
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
        };

        storage::save_invoice(&env, &invoice);
        events::invoice_created(&env, invoice_id, &freelancer, &client, amount);
        Ok(invoice_id)
    }

    /// Initializes the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if storage::get_admin(&env).is_ok() {
            panic!("Already initialized");
        }
        storage::set_admin(&env, &admin);
    }

    /// Allows the client to deposit funds into escrow for the given invoice.
    pub fn fund_invoice(env: Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
        let invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Funded) {
            panic_with_error!(&env, ContractError::InvalidInvoiceStatus);
        }

        if token_address != invoice.token {
            panic_with_error!(&env, ContractError::TokenMismatch);
        }

        let token_client = token::Client::new(&env, &invoice.token);
        token_client.transfer(&invoice.client, &env.current_contract_address(), &invoice.amount);

        storage::update_invoice_status(&env, invoice_id, storage::InvoiceStatus::Funded);
        events::invoice_funded(&env, invoice_id, &invoice.client, invoice.amount);
        Ok(())
    }

    /// Allows the freelancer to signal that work has been completed.
    pub fn mark_delivered(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.freelancer.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Delivered) {
            panic_with_error!(&env, ContractError::InvalidInvoiceStatus);
        }

        storage::update_invoice_status(&env, invoice_id, storage::InvoiceStatus::Delivered);
        events::mark_delivered(&env, invoice_id, &invoice.freelancer);
        Ok(())
    }

    /// Allows the client to approve the delivered work, authorising fund release.
    pub fn approve_payment(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Approved) {
            panic_with_error!(&env, ContractError::InvalidInvoiceStatus);
        }

        storage::update_invoice_status(&env, invoice_id, storage::InvoiceStatus::Approved);
        events::invoice_approved(&env, invoice_id, &invoice.client);
        Ok(())
    }

    /// Cancels a Pending or Funded invoice, voiding it permanently.
    pub fn cancel_invoice(env: Env, invoice_id: u64, caller: Address) -> Result<(), ContractError> {
        caller.require_auth();

        let invoice = storage::get_invoice(&env, invoice_id)?;

        if caller != invoice.freelancer && caller != invoice.client {
            panic_with_error!(&env, ContractError::UnauthorizedCaller);
        }

        if !validate_transition(&invoice.status, &InvoiceStatus::Cancelled) {
            panic_with_error!(&env, ContractError::InvalidInvoiceStatus);
        }

        storage::update_invoice_status(&env, invoice_id, storage::InvoiceStatus::Cancelled);
        events::invoice_cancelled(&env, invoice_id, &caller);
        Ok(())
    }

    /// Releases escrowed funds to the freelancer once the invoice is approved.
    pub fn release_payment(env: Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        if !validate_transition(&invoice.status, &InvoiceStatus::Completed) {
            panic_with_error!(&env, ContractError::InvalidInvoiceStatus);
        }

        let token = token::Client::new(&env, &token_address);
        token.transfer(&env.current_contract_address(), &invoice.freelancer, &invoice.amount);

        invoice.status = storage::InvoiceStatus::Completed;
        storage::save_invoice(&env, &invoice);
        events::release_payment(&env, invoice_id, &invoice.freelancer, invoice.amount);
        Ok(())
    }

    /// Returns the data for a specific invoice ID.
    pub fn get_invoice(env: Env, invoice_id: u64) -> Result<Invoice, ContractError> {
        storage::get_invoice(&env, invoice_id)
    }

    /// Returns all invoice IDs for a given freelancer.
    pub fn get_invoices_by_freelancer(env: Env, freelancer: Address) -> soroban_sdk::Vec<u64> {
        storage::get_invoices_by_freelancer(&env, &freelancer)
    }

    /// Returns all invoice IDs for a given client.
    pub fn get_invoices_by_client(env: Env, client: Address) -> soroban_sdk::Vec<u64> {
        storage::get_invoices_by_client(&env, &client)
    }

    /// Returns the total number of invoices created.
    pub fn invoice_count(env: Env) -> u64 {
        storage::get_invoice_count(&env)
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
        contract_id: &Address,
        freelancer: &Address,
        payer: &Address,
        amount: i128,
    ) -> (u64, Address) {
        let title = String::from_str(env, "Test Invoice");
        let description = String::from_str(env, "Test description");
        let token_address = setup_token(env, payer, amount);
        let invoice_id = client.create_invoice(
            freelancer,
            payer,
            &amount,
            &token_address,
            &9_999_999_999,
            &title,
            &description,
        );
        client.fund_invoice(&invoice_id, &token_address);
        let _ = contract_id; // suppress unused warning
        (invoice_id, token_address)
    }

    // ── Happy-path tests ──────────────────────────────────────────────────────

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

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description);
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

        for i in 0..5u64 {
            let invoice_id = client.create_invoice(
                &freelancer, &payer, &1000, &Address::generate(&env),
                &9_999_999_999, &title, &description,
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
        invoice_client.release_payment(&invoice_id, &token_address);

        let token_client = token::Client::new(&env, &token_address);
        assert_eq!(token_client.balance(&freelancer), amount);
        assert_eq!(token_client.balance(&contract_id), 0);

        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Completed);
    }

    // ── Error-code tests ──────────────────────────────────────────────────────

    #[test]
    fn test_invoice_not_found_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let result = client.try_get_invoice(&999);
        assert_eq!(result, Err(Ok(ContractError::InvoiceNotFound)));
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

        let result = client.try_create_invoice(&freelancer, &payer, &0, &token, &9_999_999_999, &title, &description);
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

        let result = client.try_create_invoice(&freelancer, &freelancer, &1000, &token, &9_999_999_999, &title, &description);
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
        // 257 'a' characters
        let long_desc = String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        let result = client.try_create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &long_desc);
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

        // Invoice is Pending — mark_delivered requires Funded
        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description);
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

        // Invoice is Funded — approve_payment requires Delivered
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
        let (invoice_id, token_address) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        client.mark_delivered(&invoice_id);
        // Invoice is Delivered — release_payment requires Approved
        let result = client.try_release_payment(&invoice_id, &token_address);
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

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token, &9_999_999_999, &title, &description);
        let result = client.try_fund_invoice(&invoice_id, &wrong_token);
        assert_eq!(result, Err(Ok(ContractError::TokenMismatch)));
    }

    // ── Auth tests ────────────────────────────────────────────────────────────

    #[test]
    #[should_panic]
    fn test_mark_delivered_wrong_caller() {
        let env = Env::default();
        // No mock_all_auths — auth will fail for wrong caller
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);

        env.mock_all_auths();
        let (invoice_id, _) = create_funded_invoice(&env, &client, &contract_id, &freelancer, &payer, 1000);

        // Clear auths so the next call fails
        env.set_auths(&[]);
        let _ = client.mark_delivered(&invoice_id);
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
        let _ = client.approve_payment(&invoice_id);
    }
}
