#![no_std]

mod constants;
mod events;
mod storage;

use soroban_sdk::{contract, contractimpl, token, Address, Env, String};

pub use storage::{ContractError, Invoice, InvoiceStatus};

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

        assert!(amount > 0, "Invoice amount must be greater than zero");
        assert!(freelancer != client, "Client and freelancer must be different addresses");

        if description.len() > constants::MAX_DESCRIPTION_LEN {
            return Err(ContractError::DescriptionTooLong);
        }

        let invoice_id = storage::next_invoice_id(&env);

        let invoice = Invoice {
            id: invoice_id,
            freelancer: freelancer.clone(),
            client: client.clone(),
            amount,
            token,
            deadline,
invoice
            title,
=======
            created_at: env.ledger().timestamp(), main
            description,
            status: InvoiceStatus::Pending,
        };

        storage::save_invoice(&env, &invoice);
        events::invoice_created(&env, invoice_id, &freelancer, &client, amount);
        Ok(invoice_id)
    }

    /// Allows the client to deposit funds into escrow for the given invoice.
    ///
    /// # Parameters
    /// - `invoice_id`: ID of the invoice to fund.
    /// - `token_address`: Address of the token contract to transfer from.
    ///
    /// # Errors
    /// - Panics if the caller is not the invoice client.
    /// - Panics if the invoice status is not `Pending`.
    pub fn fund_invoice(env: Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
        let invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Funded) {
            return Err(ContractError::InvalidInvoiceStatus);
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
            return Err(ContractError::InvalidInvoiceStatus);
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
            return Err(ContractError::InvalidInvoiceStatus);
        }

        storage::update_invoice_status(&env, invoice_id, storage::InvoiceStatus::Approved);

        events::invoice_approved(&env, invoice_id, &invoice.client);
        Ok(())
    }

    /// Cancels a Pending invoice, voiding it permanently.
    pub fn cancel_invoice(env: Env, invoice_id: u64, caller: Address) -> Result<(), ContractError> {
        caller.require_auth();

        let invoice = storage::get_invoice(&env, invoice_id)?;

        if caller != invoice.freelancer && caller != invoice.client {
            return Err(ContractError::UnauthorizedCaller);
        }

        if !validate_transition(&invoice.status, &InvoiceStatus::Cancelled) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        storage::update_invoice_status(&env, invoice_id, storage::InvoiceStatus::Cancelled);
        events::invoice_cancelled(&env, invoice_id, &caller);
        Ok(())
    }

    /// Raises a dispute on a Funded or Delivered invoice.
    pub fn dispute_invoice(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Disputed) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        invoice.status = InvoiceStatus::Disputed;
        storage::save_invoice(&env, &invoice);

        events::invoice_disputed(&env, invoice_id, &invoice.client);
        Ok(())
    }

    /// Releases escrowed funds to the freelancer once the invoice is approved.
    pub fn release_payment(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        if !validate_transition(&invoice.status, &InvoiceStatus::Completed) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        let token_client = token::Client::new(&env, &invoice.token);
        token_client.transfer(&env.current_contract_address(), &invoice.freelancer, &invoice.amount);

        invoice.status = InvoiceStatus::Completed;
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
}

#[cfg(test)]
mod tests {
    use super::*;
  invoice
    use soroban_sdk::{testutils::{Address as _, Ledger}, Env, String};

    fn setup_token(env: &Env) -> Address {
        let admin = Address::generate(env);
        env.register_stellar_asset_contract_v2(admin).address()
    }

    #[test]
    #[should_panic(expected = "Client and freelancer must be different addresses")]
    fn test_create_invoice_client_equals_freelancer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let title = String::from_str(&env, "Self Invoice");
        let description = String::from_str(&env, "Self-invoice");

        client.create_invoice(&freelancer, &freelancer, &1000, &Address::generate(&env), &9999999999, &title, &description);
    }
=======
    use soroban_sdk::{testutils::Address as _, Env, String};
 main

    #[test]
    fn test_create_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
 invoice
        let token_address = setup_token(&env);
        let title = String::from_str(&env, "Website Redesign");
        let description = String::from_str(&env, "Website redesign - Phase 1");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &token_address, &9999999999, &title, &description);
=======
        let description = String::from_str(&env, "Website redesign - Phase 1");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &description);
 main

        assert_eq!(invoice_id, 0);

        // Verify the invoice was stored correctly
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.freelancer, freelancer);
        assert_eq!(invoice.client, payer);
        assert_eq!(invoice.amount, 1000);
    }

    #[test]
    fn test_cancel_invoice_by_freelancer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
 invoice
        let token_address = setup_token(&env);
        let title = String::from_str(&env, "Logo Design");
        let description = String::from_str(&env, "Logo design");

        let invoice_id = client.create_invoice(&freelancer, &payer, &500, &token_address, &9999999999, &title, &description);
=======
        let description = String::from_str(&env, "Logo design");

        let invoice_id = client.create_invoice(&freelancer, &payer, &500, &description);
 main
        client.cancel_invoice(&invoice_id, &freelancer);

        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Cancelled);
    }

    #[test]
    fn test_cancel_invoice_by_client() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
 invoice
        let token_address = setup_token(&env);
        let title = String::from_str(&env, "SEO Audit");
        let description = String::from_str(&env, "SEO audit");

        let invoice_id = client.create_invoice(&freelancer, &payer, &200, &token_address, &9999999999, &title, &description);
=======
        let description = String::from_str(&env, "SEO audit");

        let invoice_id = client.create_invoice(&freelancer, &payer, &200, &description);
      main
        client.cancel_invoice(&invoice_id, &payer);

        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Cancelled);
    }

    #[test]
    #[should_panic(expected = "Only the freelancer or client can cancel the invoice")]
    fn test_cancel_invoice_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let stranger = Address::generate(&env);
      invoice
        let token_address = setup_token(&env);
        let title = String::from_str(&env, "Branding Package");
=======
 main
        let description = String::from_str(&env, "Branding package");

        let invoice_id = client.create_invoice(&freelancer, &payer, &750, &token_address, &9999999999, &title, &description);
        let _ = client.cancel_invoice(&invoice_id, &stranger);
    }

    #[test]
    #[should_panic(expected = "Invoice can only be cancelled from Pending status")]
    fn test_cancel_invoice_wrong_status() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client_contract = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
 invoice
        let token_address = setup_token(&env);
        let title = String::from_str(&env, "App Development");
        let description = String::from_str(&env, "App development");

        let invoice_id = client_contract.create_invoice(&freelancer, &payer, &3000, &token_address, &9999999999, &title, &description);
=======
        let description = String::from_str(&env, "App development");

        let invoice_id = client_contract.create_invoice(&freelancer, &payer, &3000, &description);

        // Cancel once to move it out of Pending
 main
        client_contract.cancel_invoice(&invoice_id, &freelancer);

        // Attempt to cancel again — should panic
        let _ = client_contract.cancel_invoice(&invoice_id, &freelancer);
    }

    #[test]
 invoice
    #[should_panic(expected = "Invoice can only be cancelled from Pending status")]
    fn test_cancel_invoice_from_funded() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Development services");
        let amount: i128 = 1500;

        // Deploy mock token
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);

        // Create invoice (Pending)
        let title = String::from_str(&env, "Development Services");
        let invoice_id = client.create_invoice(&freelancer, &payer, &amount, &token_address, &9999999999, &title, &description);

        // Fund it to Funded status
        client.fund_invoice(&invoice_id, &token_address);

        // Try to cancel from Funded -> should panic
        let _ = client.cancel_invoice(&invoice_id, &freelancer);
    }

    #[test]
=======
 main
    fn test_fund_invoice_happy_path() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        // Deploy the invoice contract
        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Smart contract audit");
        let amount: i128 = 5000;

        // Deploy a mock token and mint funds to the payer
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);

        // Create and fund the invoice
        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Assert status is now Funded
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Funded);

        // Assert the contract holds the escrowed tokens
        let token_client = token::Client::new(&env, &token_address);
        assert_eq!(token_client.balance(&contract_id), amount);
        assert_eq!(token_client.balance(&payer), 0);
    }
    #[test]
    fn test_invoice_count() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        assert_eq!(client.invoice_count(), 0);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);

        client.create_invoice(
            &freelancer,
            &payer,
            &1000,
            &Address::generate(&env), // token
            &9999999999, // deadline
            &String::from_str(&env, "Task 1"),
            &String::from_str(&env, "Desc 1"),
        );
        assert_eq!(client.invoice_count(), 1);

        client.create_invoice(
            &freelancer,
            &payer,
            &2000,
            &Address::generate(&env), // token
            &9999999999, // deadline
            &String::from_str(&env, "Task 2"),
            &String::from_str(&env, "Desc 2"),
        );
        assert_eq!(client.invoice_count(), 2);
    }

    #[test]
    fn test_get_invoice() {
        let env = Env::default();
        env.mock_all_auths();

 invoice
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Dispute Test");
        let description = String::from_str(&env, "Dispute test pending");

        let invoice_id = client.create_invoice(&freelancer, &payer, &100, &Address::generate(&env), &9999999999, &title, &description);
        client.dispute_invoice(&invoice_id);
    }

    // Issue #80: Negative tests for wrong-caller authorization
    #[test]
    #[should_panic]
    fn test_fund_invoice_wrong_caller() {
        let env = Env::default();
        // Do not mock all auths to test auth failure

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Test Funding");
        let description = String::from_str(&env, "Test funding");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);
=======
        if !validate_transition(&invoice.status, &InvoiceStatus::Completed) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        let token_client = token::Client::new(&env, &invoice.token);
        token_client.transfer(&env.current_contract_address(), &invoice.freelancer, &invoice.amount);

        invoice.status = InvoiceStatus::Completed;
        storage::save_invoice(&env, &invoice);
 main

        events::release_payment(&env, invoice_id, &invoice.freelancer, invoice.amount);
        Ok(())
    }

 invoice
    #[test]
    #[should_panic]
    fn test_mark_delivered_wrong_caller() {
        let env = Env::default();
        // Do not mock all auths

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Test Delivery");
        let description = String::from_str(&env, "Test delivery");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);

        // Fund the invoice first
        env.mock_all_auths(); // temporarily mock to fund
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &1000);
        client.fund_invoice(&invoice_id, &token_address);
        env.set_auths(&[]); // clear mocks

        // Try to mark delivered as client (wrong caller) - should panic
        let _ = client.mark_delivered(&invoice_id);
    }

    #[test]
    #[should_panic]
    fn test_approve_payment_wrong_caller() {
        let env = Env::default();
        // Do not mock all auths

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Test Approval");
        let description = String::from_str(&env, "Test approval");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);

        // Fund and deliver the invoice first
        env.mock_all_auths();
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &1000);
        client.fund_invoice(&invoice_id, &token_address);
        client.mark_delivered(&invoice_id);
        env.set_auths(&[]);

        // Try to approve as freelancer (wrong caller) - should panic
        let _ = client.approve_payment(&invoice_id);
    }
=======
    /// Returns the data for a specific invoice ID.
    pub fn get_invoice(env: Env, invoice_id: u64) -> Result<Invoice, ContractError> {
        storage::get_invoice(&env, invoice_id)
    }

    #[test]
    fn test_dispute_invoice_by_client() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;
 main

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
 invoice
        let title = String::from_str(&env, "Test Double Funding");
        let description = String::from_str(&env, "Test double funding");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);
=======
        let description = String::from_str(&env, "Dispute case");
        let amount: i128 = 2000;
 main

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
 invoice
        token_admin_client.mint(&payer, &2000); // mint extra

        client.fund_invoice(&invoice_id, &token_address);
        // Try to fund again - should panic
        let _ = client.fund_invoice(&invoice_id, &token_address);
    }

    #[test]
    #[should_panic(expected = "Invoice must be in Funded status")]
    fn test_mark_delivered_pending_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Test Mark Delivered");
        let description = String::from_str(&env, "Test mark delivered on pending");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);

        // Try to mark delivered on pending - should panic
        let _ = client.mark_delivered(&invoice_id);
    }

    #[test]
    #[should_panic(expected = "Invoice must be in Delivered status")]
    fn test_approve_payment_funded_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Test Approve on Funded");
        let description = String::from_str(&env, "Test approve on funded");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);
=======
        token_admin_client.mint(&payer, &amount);

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Dispute from Funded status
        invoice_client.dispute_invoice(&invoice_id, &payer);
 main

        let invoice = invoice_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, storage::InvoiceStatus::Disputed);
    }

    #[test]
 invoice
    #[should_panic(expected = "Invoice must be in Approved status")]
    fn test_release_payment_delivered_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Test Release on Delivered");
        let description = String::from_str(&env, "Test release on delivered");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &1000);
        client.fund_invoice(&invoice_id, &token_address);
        client.mark_delivered(&invoice_id);

        // Try to release on delivered (not approved) - should panic
        let _ = client.release_payment(&invoice_id, &token_address);
    }
=======
    #[should_panic(expected = "Invoice can only be disputed from")]
    fn test_dispute_invoice_invalid_status() {
        use soroban_sdk::testutils::Address as _;
 main

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Status dispute");

 invoice
        // Deploy token and mint to payer
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);

        let token_client = token::Client::new(&env, &token_address);

        // Step 1: Create invoice
        let title = String::from_str(&env, "Full Escrow Flow");
        let invoice_id = client.create_invoice(&freelancer, &payer, &amount, &token_address, &9999999999, &title, &description);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Pending);

        // Step 2: Fund invoice
        client.fund_invoice(&invoice_id, &token_address);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Funded);
        assert_eq!(token_client.balance(&contract_id), amount);
        assert_eq!(token_client.balance(&payer), 0);

        // Step 3: Mark delivered
        client.mark_delivered(&invoice_id);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Delivered);

        // Step 4: Approve payment
        client.approve_payment(&invoice_id);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Approved);

        // Step 5: Release payment
        client.release_payment(&invoice_id, &token_address);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Completed);

        // Assert final balances
        assert_eq!(token_client.balance(&contract_id), 0);
        assert_eq!(token_client.balance(&freelancer), amount);
        assert_eq!(token_client.balance(&payer), 0);
    }

    // Issue #83: Test for create_invoice with duplicate IDs (regression)
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

        let mut ids: soroban_sdk::Vec<u64> = soroban_sdk::Vec::new(&env);

        for i in 0..10u64 {
            let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &Address::generate(&env), &9999999999, &title, &description);
            assert_eq!(invoice_id, i);
            // Check not already in ids
            let mut is_unique = true;
            for existing_id in ids.iter() {
                if existing_id == invoice_id {
                    is_unique = false;
                    break;
                }
            }
            assert!(is_unique, "Duplicate ID found: {}", invoice_id);
            ids.push_back(invoice_id);
        }

        assert_eq!(client.invoice_count(), 10);
=======
        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &100, &description);

        // It is Pending here, not Funded/Delivered.
        let _ = invoice_client.dispute_invoice(&invoice_id, &freelancer);
 main
    }
}
