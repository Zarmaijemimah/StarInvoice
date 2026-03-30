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
    ) -> Result<u64, ContractError> {
        escrow::create_invoice(&env, freelancer, client, amount, token, deadline, title, description)
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
