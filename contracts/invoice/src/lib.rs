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
            title,
            created_at: env.ledger().timestamp(),
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
    /// - Returns `InvalidInvoiceStatus` if the caller is not the invoice client.
    /// - Returns `InvalidInvoiceStatus` if the invoice status is not `Pending`.
    /// - Returns `TokenMismatch` if the provided token does not match the invoice's token.
    pub fn fund_invoice(env: Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
        let invoice = storage::get_invoice(&env, invoice_id)?;

        invoice.client.require_auth();

        if !validate_transition(&invoice.status, &InvoiceStatus::Funded) {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        // Validate that the provided token matches the invoice's token
        if token_address != invoice.token {
            return Err(ContractError::TokenMismatch);
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
    ///
    /// # Parameters
    /// - `invoice_id`: ID of the invoice to approve.
    ///
    /// # Errors
    /// - Returns error if invoice is not found.
    /// - Panics if the caller is not the invoice client.
    /// - Panics if the invoice status is not `Delivered`.
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

    /// Releases escrowed funds to the freelancer once the invoice is approved.
    ///
    /// # Parameters
    /// - `invoice_id`: ID of the invoice to settle.
    /// - `token_address`: Address of the token contract to transfer from.
    ///
    /// # Errors
    /// - Returns error if invoice is not found.
    /// - Panics if the invoice status is not `Approved`.
    pub fn release_payment(env: Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
        let mut invoice = storage::get_invoice(&env, invoice_id)?;

        assert!(
            invoice.status == storage::InvoiceStatus::Approved,
            "Invoice must be in Approved status"
        );

        let token = token::Client::new(&env, &token_address);
        token.transfer(&env.current_contract_address(), &invoice.freelancer, &invoice.amount);

        invoice.status = storage::InvoiceStatus::Completed;
        storage::save_invoice(&env, &invoice);

        events::release_payment(&env, invoice_id, &invoice.freelancer, invoice.amount);
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

    /// Initializes the contract with an admin address.
    /// This must be called once during contract deployment.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        storage::set_admin(&env, &admin);
        events::admin_initialized(&env, &admin);
        Ok(())
    }

    /// Updates the contract admin address.
    /// Only the current admin can call this function.
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        let current_admin = storage::get_admin(&env)?;
        current_admin.require_auth();

        storage::set_admin(&env, &new_admin);
        events::admin_changed(&env, &current_admin, &new_admin);
        Ok(())
    }

    /// Resolves a disputed invoice by awarding it to the winner.
    /// Only the admin can call this function.
    /// This transfers the escrowed funds to the winner and marks the invoice as completed.
    pub fn resolve_dispute(env: Env, invoice_id: u64, winner: Address) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env)?;
        admin.require_auth();

        let invoice = storage::get_invoice(&env, invoice_id)?;

        // Verify that the dispute exists
        let mut dispute = storage::get_dispute(&env, invoice_id)?;

        if invoice.status != InvoiceStatus::Disputed {
            return Err(ContractError::InvalidInvoiceStatus);
        }

        // Verify the winner is either the freelancer or the client
        if winner != invoice.freelancer && winner != invoice.client {
            return Err(ContractError::UnauthorizedCaller);
        }

        // Update dispute status
        dispute.resolved = true;
        dispute.winner = Some(winner.clone());
        storage::save_dispute(&env, &dispute);

        // Transfer funds to the winner
        let token_client = token::Client::new(&env, &invoice.token);
        token_client.transfer(&env.current_contract_address(), &winner, &invoice.amount);

        // Mark invoice as completed
        storage::update_invoice_status(&env, invoice_id, InvoiceStatus::Completed);

        events::dispute_resolved(&env, invoice_id, &winner);
        Ok(())
    }
}

    #[test]
    fn test_invoice_not_found() {
        let env = Env::default();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let result = client.try_get_invoice(&999);
        match result {
            Err(Ok(errors)) => {
                assert_eq!(errors, ContractError::InvoiceNotFound.into());
            }
            _ => panic!("Expected InvoiceNotFound error"),
        }
    }

    #[test]
    fn test_mark_delivered_happy_path() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Development work");
        let amount: i128 = 2000;

        // Create invoice
        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);

        // Fund the invoice
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Mark as delivered
        invoice_client.mark_delivered(&invoice_id);

        // Assert status is now Delivered
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Delivered);
    }

    #[test]
    #[should_panic(expected = "Invoice must be in Funded status")]
    fn test_mark_delivered_wrong_status() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Test work");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &description);
        
        // Try to mark delivered without funding first
        let _ = client.mark_delivered(&invoice_id);
    }

    #[test]
    #[should_panic(expected = "Invoice must be in Funded status")]
    fn test_mark_delivered_from_cancelled_status() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Test work");

        let invoice_id = client.create_invoice(&freelancer, &payer, &1000, &description);
        client.cancel_invoice(&invoice_id, &freelancer);
        
        // Try to mark delivered after cancellation
        let _ = client.mark_delivered(&invoice_id);
    }

    #[test]
    #[should_panic]
    fn test_mark_delivered_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let stranger = Address::generate(&env);
        let description = String::from_str(&env, "Test work");
        let amount: i128 = 1000;

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);

        // Fund the invoice
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Try to mark delivered as stranger (not freelancer)
        env.mock_all_auths_allowing_non_root_auth();
        let _ = invoice_client.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &stranger,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "mark_delivered",
                args: (invoice_id,).into_val(&env),
                sub_invokes: &[],
            },
        }]).mark_delivered(&invoice_id);
    }

    #[test]
    fn test_approve_payment_happy_path() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Consulting services");
        let amount: i128 = 3000;

        // Create and fund invoice
        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Mark as delivered
        invoice_client.mark_delivered(&invoice_id);

        // Approve payment
        invoice_client.approve_payment(&invoice_id);

        // Assert status is now Approved
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Approved);
    }

    #[test]
    #[should_panic(expected = "Invoice must be in Delivered status")]
    fn test_approve_payment_wrong_status() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Test work");
        let amount: i128 = 1000;

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);

        // Try to approve without marking delivered first
        let _ = invoice_client.approve_payment(&invoice_id);
    }

    #[test]
    #[should_panic]
    fn test_approve_payment_wrong_caller() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let stranger = Address::generate(&env);
        let description = String::from_str(&env, "Test work");
        let amount: i128 = 1000;

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);
        invoice_client.mark_delivered(&invoice_id);

        // Try to approve as stranger (not client)
        env.mock_all_auths_allowing_non_root_auth();
        let _ = invoice_client.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &stranger,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "approve_payment",
                args: (invoice_id,).into_val(&env),
                sub_invokes: &[],
            },
        }]).approve_payment(&invoice_id);
    }

    #[test]
    fn test_release_payment_happy_path() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Final payment test");
        let amount: i128 = 4000;

        // Create, fund, deliver, and approve invoice
        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);
        invoice_client.mark_delivered(&invoice_id);
        invoice_client.approve_payment(&invoice_id);

        // Release payment
        invoice_client.release_payment(&invoice_id, &token_address);

        // Assert status is now Completed
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Completed);

        // Assert freelancer received the tokens
        let token_client = token::Client::new(&env, &token_address);
        assert_eq!(token_client.balance(&freelancer), amount);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    #[should_panic(expected = "Invoice must be in Approved status")]
    fn test_release_payment_wrong_status() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Test work");
        let amount: i128 = 1000;

        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        invoice_client.fund_invoice(&invoice_id, &token_address);
        invoice_client.mark_delivered(&invoice_id);

        // Try to release without approval
        let _ = invoice_client.release_payment(&invoice_id, &token_address);
    }

    #[test]
    fn test_end_to_end_escrow_flow() {
        use soroban_sdk::testutils::Address as _;
        use soroban_sdk::token;

        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let invoice_client = InvoiceContractClient::new(&env, &contract_id);

        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let description = String::from_str(&env, "Complete escrow flow test");
        let amount: i128 = 5000;

        // Setup token
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        token_admin_client.mint(&payer, &amount);
        let token_client = token::Client::new(&env, &token_address);

        // Step 1: Create invoice
        let invoice_id = invoice_client.create_invoice(&freelancer, &payer, &amount, &description);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Pending);

        // Step 2: Fund invoice
        invoice_client.fund_invoice(&invoice_id, &token_address);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Funded);
        assert_eq!(token_client.balance(&contract_id), amount);
        assert_eq!(token_client.balance(&payer), 0);

        // Step 3: Mark delivered
        invoice_client.mark_delivered(&invoice_id);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Delivered);

        // Step 4: Approve payment
        invoice_client.approve_payment(&invoice_id);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Approved);

        // Step 5: Release payment
        invoice_client.release_payment(&invoice_id, &token_address);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Completed);
        assert_eq!(token_client.balance(&freelancer), amount);
        assert_eq!(token_client.balance(&contract_id), 0);
    }
}

