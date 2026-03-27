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
        description: String,
    ) -> u64 {
        freelancer.require_auth();

        assert!(freelancer != client, "Client and freelancer must be different addresses");

        let invoice_id = storage::next_invoice_id(&env);

        let invoice = Invoice {
            id: invoice_id,
            freelancer: freelancer.clone(),
            client: client.clone(),
            amount,
            token,
            deadline,
            created_at: env.ledger().timestamp(),
            description,
            status: InvoiceStatus::Pending,
        };

        storage::save_invoice(&env, &invoice);
        events::invoice_created(&env, invoice_id, &freelancer, &client, amount);
        invoice_id
    }

    /// Allows the client to deposit funds into escrow for the given invoice.
    pub fn fund_invoice(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Funded) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        let token_client = token::Client::new(&env, &invoice.token);
        token_client.transfer(&invoice.client, &env.current_contract_address(), &invoice.amount);

        invoice.status = InvoiceStatus::Funded;
        storage::save_invoice(&env, &invoice);

        events::invoice_funded(&env, invoice_id, &invoice.client, invoice.amount);
        Ok(())
    }

    /// Allows the freelancer to signal that work has been completed.
    pub fn mark_delivered(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.freelancer.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Delivered) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        invoice.status = InvoiceStatus::Delivered;
        storage::save_invoice(&env, &invoice);

        events::mark_delivered(&env, invoice_id, &invoice.freelancer);
        Ok(())
    }

    /// Allows the client to approve the delivered work, authorising fund release.
    pub fn approve_payment(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Approved) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        invoice.status = InvoiceStatus::Approved;
        storage::save_invoice(&env, &invoice);

        events::invoice_approved(&env, invoice_id, &invoice.client);
        Ok(())
    }

    /// Cancels a Pending invoice, voiding it permanently.
    pub fn cancel_invoice(env: Env, invoice_id: u64, caller: Address) -> Result<(), ContractError> {
        caller.require_auth();

        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        if caller != invoice.freelancer && caller != invoice.client {
            return Err(ContractError::UnauthorizedCaller);
        }

        if !validate_transition(&invoice.status, &InvoiceStatus::Cancelled) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        invoice.status = InvoiceStatus::Cancelled;
        storage::save_invoice(&env, &invoice);
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
    ///
    /// # Parameters
    /// - `invoice_id`: ID of the invoice to settle.
    ///
    /// # Errors
    /// - Panics if the invoice status is not `Approved`.
    ///
    /// # TODO
    /// Not yet implemented. See: <https://github.com/your-org/StarInvoice/issues/4>
    pub fn release_payment(env: Env, invoice_id: u64) -> Result<(), ContractError> {
        let invoice = storage::get_invoice(&env, invoice_id)?;
        
        // Emit the event
        events::invoice_released(&env, invoice_id, &invoice.freelancer, invoice.amount);

        todo!("release_payment token transfer not yet implemented")
    }

    /// Allows either party to dispute an invoice.
    ///
    /// # Parameters
    /// - `invoice_id`: ID of the invoice to dispute.
    /// - `caller`: Address of the party raising the dispute (freelancer or client).
    ///
    /// # Errors
    /// - Panics if the invoice status is not `Funded` or `Delivered`.
    /// - Panics if `caller` is neither the freelancer nor the client.
    pub fn dispute_invoice(env: Env, invoice_id: u64, caller: Address) -> Result<(), ContractError> {
        caller.require_auth();

        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        assert!(
            invoice.status == storage::InvoiceStatus::Funded || invoice.status == storage::InvoiceStatus::Delivered,
            "Invoice can only be disputed from Funded or Delivered status"
        );

        assert!(
            caller == invoice.freelancer || caller == invoice.client,
            "Only the freelancer or client can dispute the invoice"
        );

        invoice.status = storage::InvoiceStatus::Disputed;
        storage::save_invoice(&env, &invoice);
        events::invoice_disputed(&env, invoice_id, &caller);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    #[test]
    fn test_create_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Website redesign - Phase 1");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &description);

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
        let description = String::from_str(&env, "Logo design");

        let invoice_id = client.create_invoice(&freelancer, &payer, &500, &description);
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
        let description = String::from_str(&env, "SEO audit");

        let invoice_id = client.create_invoice(&freelancer, &payer, &200, &description);
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
        let description = String::from_str(&env, "Branding package");

        let invoice_id = client.create_invoice(&freelancer, &payer, &750, &description);
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
        let description = String::from_str(&env, "App development");

        let invoice_id = client_contract.create_invoice(&freelancer, &payer, &3000, &description);

        // Cancel once to move it out of Pending
        client_contract.cancel_invoice(&invoice_id, &freelancer);

        // Attempt to cancel again — should panic
        let _ = client_contract.cancel_invoice(&invoice_id, &freelancer);
    }

    #[test]
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
            &String::from_str(&env, "Desc 1"),
        );
        assert_eq!(client.invoice_count(), 1);

        client.create_invoice(
            &freelancer,
            &payer,
            &2000,
            &String::from_str(&env, "Desc 2"),
        );
        assert_eq!(client.invoice_count(), 2);
    }

    #[test]
    fn test_get_invoice() {
        let env = Env::default();
        env.mock_all_auths();

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

    #[test]
    fn test_dispute_invoice_by_client() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Dispute case");
        let amount: i128 = 2000;

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Dispute from Funded status
        invoice_client.dispute_invoice(&invoice_id, &payer);

        let invoice = invoice_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, storage::InvoiceStatus::Disputed);
    }

    #[test]
    #[should_panic(expected = "Invoice can only be disputed from")]
    fn test_dispute_invoice_invalid_status() {
        use soroban_sdk::testutils::Address as _;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Status dispute");

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &100, &description);

        // It is Pending here, not Funded/Delivered.
        let _ = invoice_client.dispute_invoice(&invoice_id, &freelancer);
    }
}
