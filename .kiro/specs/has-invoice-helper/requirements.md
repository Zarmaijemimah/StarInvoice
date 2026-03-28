# Requirements Document

## Introduction

The invoice smart contract currently has no way to check whether an invoice exists without panicking. The `get_invoice` function in `storage.rs` returns an error via `Result`, but callers that need a simple boolean existence check must handle the full `Result` type or risk a panic in contexts where the invoice may legitimately be absent. This feature adds a `has_invoice` helper to `storage.rs` and updates `get_invoice` in `lib.rs` to use it, ensuring safe, non-panicking existence checks throughout the contract.

## Glossary

- **Storage**: The `storage.rs` module responsible for all on-chain read/write operations for invoices.
- **Contract**: The `InvoiceContract` defined in `lib.rs` that exposes the public API of the Soroban smart contract.
- **Invoice**: An on-chain data structure representing a payment agreement between a freelancer and a client.
- **invoice_id**: A `u64` value that uniquely identifies an invoice in persistent storage.
- **ContractError**: The `#[contracterror]` enum in `storage.rs` used to signal recoverable error conditions.
- **has_invoice**: The new public helper function `pub fn has_invoice(env: &Env, invoice_id: u64) -> bool` to be added to `storage.rs`.
- **get_invoice**: The existing function in `storage.rs` that retrieves an invoice by ID, returning `Result<Invoice, ContractError>`.

## Requirements

### Requirement 1: has_invoice Helper Function

**User Story:** As a smart contract developer, I want a `has_invoice` function in `storage.rs`, so that I can check invoice existence without handling a `Result` or risking a panic.

#### Acceptance Criteria

1. THE Storage SHALL expose a public function `has_invoice(env: &Env, invoice_id: u64) -> bool`.
2. WHEN `has_invoice` is called with an `invoice_id` that exists in persistent storage, THE Storage SHALL return `true`.
3. WHEN `has_invoice` is called with an `invoice_id` that does not exist in persistent storage, THE Storage SHALL return `false`.
4. THE `has_invoice` function SHALL NOT panic under any input.
5. THE `has_invoice` function SHALL NOT modify any storage state.

### Requirement 2: get_invoice Uses has_invoice Internally

**User Story:** As a smart contract developer, I want `get_invoice` to use `has_invoice` for its existence check, so that the existence logic is defined in one place and remains consistent.

#### Acceptance Criteria

1. WHEN `get_invoice` is called with an `invoice_id` that does not exist, THE Storage SHALL return `Err(ContractError::InvoiceNotFound)`.
2. WHEN `get_invoice` is called with an `invoice_id` that exists, THE Storage SHALL return `Ok(Invoice)` with the correct invoice data.
3. THE `get_invoice` function SHALL delegate its existence check to `has_invoice` rather than duplicating the storage lookup logic.

### Requirement 3: Contract get_invoice Returns Result

**User Story:** As a contract caller, I want `get_invoice` on the `InvoiceContract` to return a `Result<Invoice, ContractError>` instead of panicking, so that missing invoices are handled gracefully at the API boundary.

#### Acceptance Criteria

1. THE Contract SHALL expose `get_invoice(env: Env, invoice_id: u64) -> Result<Invoice, ContractError>` as a public contract function.
2. WHEN `get_invoice` is called with a valid `invoice_id`, THE Contract SHALL return `Ok(Invoice)`.
3. WHEN `get_invoice` is called with an `invoice_id` that does not exist, THE Contract SHALL return `Err(ContractError::InvoiceNotFound)`.
4. THE Contract `get_invoice` function SHALL NOT panic for any `invoice_id` input.

### Requirement 4: Round-Trip Consistency

**User Story:** As a smart contract developer, I want `has_invoice` and `get_invoice` to be consistent with each other, so that an invoice reported as existing by `has_invoice` can always be retrieved by `get_invoice`.

#### Acceptance Criteria

1. FOR ALL `invoice_id` values, IF `has_invoice` returns `true`, THEN THE Storage `get_invoice` SHALL return `Ok(Invoice)` for the same `invoice_id`.
2. FOR ALL `invoice_id` values, IF `has_invoice` returns `false`, THEN THE Storage `get_invoice` SHALL return `Err(ContractError::InvoiceNotFound)` for the same `invoice_id`.
