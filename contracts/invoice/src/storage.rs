use soroban_sdk::{contracterror, contracttype, Address, Env, String};
use crate::constants::{TTL_THRESHOLD, TTL_EXTEND_TO, MAX_DESCRIPTION_LEN};

/// Contract-level errors returned by state-changing functions.
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContractError {
    /// The invoice does not exist.
    InvoiceNotFound = 1,
    /// The invoice is not in the required status for this operation.
    InvalidInvoiceStatus = 2,
    /// The caller is not authorised to perform this operation.
    UnauthorizedCaller = 3,
    /// The description exceeds the maximum allowed length.
    DescriptionTooLong = 4,
    /// The token provided does not match the invoice's token.
    TokenMismatch = 5,
}

/// Represents the lifecycle state of an invoice.
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum InvoiceStatus {
    /// Invoice created, awaiting client funding.
    Pending,
    /// Client has deposited funds into escrow.
    Funded,
    /// Freelancer has marked work as delivered.
    Delivered,
    /// Client disputes the invoice.
    Disputed,
    /// Client has approved the delivery.
    Approved,
    /// Funds have been released to the freelancer.
    Completed,
    /// Invoice has been voided by the freelancer or client.
    Cancelled,
    /// Invoice is currently under dispute.
    Disputed,
}

/// Core invoice data structure stored on-chain.
#[contracttype]
#[derive(Clone)]
pub struct Invoice {
    /// Unique numeric identifier for this invoice.
    pub id: u64,
    /// Address of the freelancer who created the invoice.
    pub freelancer: Address,
    /// Address of the client responsible for funding.
    pub client: Address,
    /// Payment amount in the smallest token unit (stroops).
    pub amount: i128,
    /// Short human-readable title for the invoice.
    pub title: String,
    /// Human-readable description of the work to be performed.
    pub description: String,
    /// Address of the token contract used for payment.
    pub token: Address,
    /// Unix timestamp after which the invoice can no longer be funded.
    pub deadline: u64,
    /// Unix timestamp when the invoice was created.
    pub created_at: u64,
    /// Current state of the invoice in the escrow lifecycle.
    pub status: InvoiceStatus,
}

#[contracttype]
enum DataKey {
    Invoice(u64),
    InvoiceCount,
    InvoicesByFreelancer(Address),
    InvoicesByClient(Address),
}

/// Returns the current invoice count.
pub fn get_invoice_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::InvoiceCount)
        .unwrap_or(0)
}

/// Returns the next available invoice ID and increments the counter.
/// 
/// NOTE: Soroban transactions are atomic at the transaction level. Each transaction
/// is executed in isolation and either fully succeeds or fully fails. Therefore,
/// even though this function performs two storage operations (read and write),
/// they are guaranteed to be atomic within a single transaction. No race condition
/// can occur because concurrent transactions are serialized by the ledger.
pub fn next_invoice_id(env: &Env) -> u64 {
    let count: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::InvoiceCount)
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&DataKey::InvoiceCount, &(count + 1));
    count
}

/// Persists an invoice to on-chain storage, keyed by its ID.
pub fn save_invoice(env: &Env, invoice: &Invoice) {
    let key = DataKey::Invoice(invoice.id);
    env.storage().persistent().set(&key, invoice);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);

    // Update freelancer index
    let freelancer_key = DataKey::InvoicesByFreelancer(invoice.freelancer.clone());
    let mut freelancer_invoices = env
        .storage()
        .persistent()
        .get::<_, soroban_sdk::Vec<u64>>(&freelancer_key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));
    if !freelancer_invoices.iter().any(|id| id == invoice.id) {
        freelancer_invoices.push_back(invoice.id);
        env.storage()
            .persistent()
            .set(&freelancer_key, &freelancer_invoices);
    }

    // Update client index
    let client_key = DataKey::InvoicesByClient(invoice.client.clone());
    let mut client_invoices = env
        .storage()
        .persistent()
        .get::<_, soroban_sdk::Vec<u64>>(&client_key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));
    if !client_invoices.iter().any(|id| id == invoice.id) {
        client_invoices.push_back(invoice.id);
        env.storage()
            .persistent()
            .set(&client_key, &client_invoices);
    }
}

/// Retrieves an invoice by ID, returning an error if not found.
pub fn get_invoice(env: &Env, invoice_id: u64) -> Result<Invoice, ContractError> {
    let key = DataKey::Invoice(invoice_id);
    let invoice = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::InvoiceNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    Ok(invoice)
}

/// Returns all invoice IDs for a given freelancer.
pub fn get_invoices_by_freelancer(env: &Env, freelancer: &Address) -> soroban_sdk::Vec<u64> {
    let key = DataKey::InvoicesByFreelancer(freelancer.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env))
}

/// Returns all invoice IDs for a given client.
pub fn get_invoices_by_client(env: &Env, client: &Address) -> soroban_sdk::Vec<u64> {
    let key = DataKey::InvoicesByClient(client.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env))
}
