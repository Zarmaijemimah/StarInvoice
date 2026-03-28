# PR Title

feat: add has_invoice helper for safe invoice existence checks

---

# PR Description

## Summary

Adds a `has_invoice(env, invoice_id) -> bool` helper to `storage.rs` to allow safe, non-panicking invoice existence checks. Updates `get_invoice` to delegate its existence check to `has_invoice`, ensuring the logic lives in one place and the contract API returns a `Result` instead of panicking on missing invoices.

## Changes

- Adds `pub fn has_invoice(env: &Env, invoice_id: u64) -> bool` to `storage.rs` — no panics, no state mutations
- Updates `get_invoice` in `storage.rs` to use `has_invoice` internally, returning `Err(ContractError::InvoiceNotFound)` when absent
- Contract-level `get_invoice` now returns `Result<Invoice, ContractError>` instead of panicking

## Motivation

Previously, callers had no way to check invoice existence without handling a full `Result` or risking a panic. This change makes existence checks safe and consistent across the contract.
