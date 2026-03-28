# PR Title

test: add full escrow flow integration test

---

# PR Description

## Summary

Adds a single end-to-end integration test that walks through the complete invoice escrow lifecycle: `create_invoice` → `fund_invoice` → `mark_delivered` → `approve_payment` → `release_payment`. This gives confidence that all five steps work correctly together and that funds move as expected.

## Changes

- Adds `test_full_escrow_flow` integration test covering all five contract calls in sequence
- Asserts `InvoiceStatus` at each step: `Pending → Funded → Delivered → Approved → Completed`
- Asserts contract holds the escrowed amount after `fund_invoice`
- Asserts freelancer receives the full amount and contract balance hits zero after `release_payment`

## Testing

The new test lives in `contracts/invoice/tests/` and can be run with:

```bash
cargo test test_full_escrow_flow
```
