use soroban_sdk::{panic_with_error, token, Address, Env, String};

#[allow(clippy::wildcard_imports)] // constants module is a flat list of pub consts
use crate::constants::*;
use crate::storage::{self, ContractError, InvoiceStatus};
use crate::{events, validate_transition, Invoice};

pub fn create_invoice(
    env: &Env,
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
        panic_with_error!(env, ContractError::InvalidAmount);
    }

    if amount > MAX_INVOICE_AMOUNT {
        panic_with_error!(env, ContractError::AmountExceedsMaximum);
    }

    if freelancer == client {
        panic_with_error!(env, ContractError::InvalidParties);
    }

    if description.len() > MAX_DESCRIPTION_LEN.try_into().unwrap() {
        panic_with_error!(env, ContractError::DescriptionTooLong);
    }

    let invoice_id = storage::next_invoice_id(env);

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

    storage::save_invoice(env, &invoice);
    events::invoice_created(env, invoice_id, &freelancer, &client, amount);
    Ok(invoice_id)
}

pub fn fund_invoice(env: &Env, invoice_id: u64, token_address: Address) -> Result<(), ContractError> {
    let invoice = storage::get_invoice(env, invoice_id)?;

    invoice.client.require_auth();

    if !validate_transition(&invoice.status, &InvoiceStatus::Funded) {
        panic_with_error!(env, ContractError::InvalidInvoiceStatus);
    }

    if token_address != invoice.token {
        panic_with_error!(env, ContractError::TokenMismatch);
    }

    let token_client = token::Client::new(env, &invoice.token);
    token_client.transfer(&invoice.client, &env.current_contract_address(), &invoice.amount);

    storage::update_invoice_status(env, invoice_id, InvoiceStatus::Funded);
    events::invoice_funded(env, invoice_id, &invoice.client, invoice.amount);
    Ok(())
}

pub fn mark_delivered(env: &Env, invoice_id: u64) -> Result<(), ContractError> {
    let invoice = storage::get_invoice(env, invoice_id)?;

    invoice.freelancer.require_auth();

    if !validate_transition(&invoice.status, &InvoiceStatus::Delivered) {
        panic_with_error!(env, ContractError::InvalidInvoiceStatus);
    }

    storage::update_invoice_status(env, invoice_id, InvoiceStatus::Delivered);
    events::mark_delivered(env, invoice_id, &invoice.freelancer);
    Ok(())
}

pub fn approve_payment(env: &Env, invoice_id: u64) -> Result<(), ContractError> {
    let invoice = storage::get_invoice(env, invoice_id)?;

    invoice.client.require_auth();

    if !validate_transition(&invoice.status, &InvoiceStatus::Approved) {
        panic_with_error!(env, ContractError::InvalidInvoiceStatus);
    }

    storage::update_invoice_status(env, invoice_id, InvoiceStatus::Approved);
    events::invoice_approved(env, invoice_id, &invoice.client);
    Ok(())
}

pub fn cancel_invoice(env: &Env, invoice_id: u64, caller: Address) -> Result<(), ContractError> {
    caller.require_auth();

    let invoice = storage::get_invoice(env, invoice_id)?;

    if caller != invoice.freelancer && caller != invoice.client {
        panic_with_error!(env, ContractError::UnauthorizedCaller);
    }

    if !validate_transition(&invoice.status, &InvoiceStatus::Cancelled) {
        panic_with_error!(env, ContractError::InvalidInvoiceStatus);
    }

    storage::update_invoice_status(env, invoice_id, InvoiceStatus::Cancelled);
    events::invoice_cancelled(env, invoice_id, &caller);
    Ok(())
}

pub fn release_payment(env: &Env, invoice_id: u64) -> Result<(), ContractError> {
    let mut invoice = storage::get_invoice(env, invoice_id)?;

    if !validate_transition(&invoice.status, &InvoiceStatus::Completed) {
        panic_with_error!(env, ContractError::InvalidInvoiceStatus);
    }

    let token_client = token::Client::new(env, &invoice.token);
    token_client.transfer(&env.current_contract_address(), &invoice.freelancer, &invoice.amount);

    invoice.status = InvoiceStatus::Completed;
    storage::save_invoice(env, &invoice);
    events::release_payment(env, invoice_id, &invoice.freelancer, invoice.amount);
    Ok(())
}

pub fn dispute_invoice(env: &Env, invoice_id: u64) -> Result<(), ContractError> {
    let invoice = storage::get_invoice(env, invoice_id)?;

    invoice.client.require_auth();

    if !validate_transition(&invoice.status, &InvoiceStatus::Disputed) {
        panic_with_error!(env, ContractError::InvalidInvoiceStatus);
    }

    storage::update_invoice_status(env, invoice_id, InvoiceStatus::Disputed);
    events::invoice_disputed(env, invoice_id, &invoice.client);
    Ok(())
}
