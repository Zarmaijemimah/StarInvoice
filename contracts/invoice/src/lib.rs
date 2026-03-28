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

#[cfg(test)]
mod tests {
    // Tests are maintained in separate test files under contracts/invoice/tests/
}

