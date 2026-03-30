use soroban_sdk::{Address, Env, Vec};

use crate::storage::{self, ContractError, Invoice};

pub fn get_invoice(env: &Env, invoice_id: u64) -> Result<Invoice, ContractError> {
    storage::get_invoice(env, invoice_id)
}

pub fn get_invoices_by_freelancer(env: &Env, freelancer: &Address) -> Vec<u64> {
    storage::get_invoices_by_freelancer(env, freelancer)
}

pub fn get_invoices_by_client(env: &Env, client: &Address) -> Vec<u64> {
    storage::get_invoices_by_client(env, client)
}

pub fn invoice_count(env: &Env) -> u64 {
    storage::get_invoice_count(env)
}

/// Returns all invoices whose `amount` falls within `[min, max]` (inclusive).
///
/// # Trade-off: full scan vs sorted index
///
/// This implementation performs a **full sequential scan** over every invoice ID
/// from 0 to `invoice_count - 1`, loading each invoice from persistent storage
/// and filtering by amount.
///
/// **Why not a sorted index?**
/// Soroban persistent storage is a key-value store with no native range queries.
/// Maintaining a sorted index (e.g. a `Vec<(i128, u64)>` ordered by amount) would
/// require re-sorting or insertion-sorting on every `create_invoice` call, which
/// costs additional CPU instructions and storage writes per invoice creation.
/// For the current scale of StarInvoice (hundreds to low thousands of invoices),
/// the full scan is simpler, cheaper to maintain, and easier to audit.
///
/// **When to reconsider:**
/// If the invoice count grows into the tens of thousands, the per-call ledger
/// instruction budget may be exceeded. At that point, introduce a
/// `AmountIndex(i128) -> Vec<u64>` bucket map or an off-chain indexer (see
/// `docs/indexing-events.md`) and remove this function from the contract.
pub fn get_invoices_by_amount_range(env: &Env, min: i128, max: i128) -> Vec<Invoice> {
    let count = storage::get_invoice_count(env);
    let mut results: Vec<Invoice> = Vec::new(env);
    for id in 0..count {
        if let Ok(invoice) = storage::get_invoice(env, id) {
            if invoice.amount >= min && invoice.amount <= max {
                results.push_back(invoice);
            }
        }
    }
    results
}
