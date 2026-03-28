# test: verify unique incrementing invoice IDs

## Description

Adds a test to verify that creating multiple invoices always produces unique, incrementing IDs.

The test creates 10 invoices in a loop and asserts each one receives a sequential ID from 0 to 9, with no duplicates.

## Changes

- Added `test_create_invoice_unique_ids` to `contracts/invoice/tests/test_create_invoice.rs`

## Acceptance Criteria

- [x] Creates 10 invoices in a loop
- [x] Asserts each invoice has a unique ID
- [x] Asserts IDs are sequential from 0 to 9

## Labels

`test`, `bug`
