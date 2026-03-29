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
