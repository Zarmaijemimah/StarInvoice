# PR Title

feat: add `refund_client` function to return escrowed funds to client

---

# PR Description

## Summary

Adds a `refund_client(env, invoice_id)` function to the `InvoiceContract` that allows escrowed funds to be returned to the client when an invoice is cancelled after funding or when a dispute is resolved in the client's favour. Previously, there was no path to recover funds once they entered escrow in these scenarios.

## Changes

- **New `InvoiceStatus::Refunded` variant** — terminal state representing a completed refund, distinct from `Completed` (freelancer paid) and `Cancelled` (no funds held).
- **`refund_client(env, invoice_id)`** — transfers the full escrowed amount back to the client. Only callable when the invoice status is `Cancelled` (post-funding) or `Disputed`. Sets status to `Refunded` on success.
- **`validate_transition` updated** — permits `Cancelled → Refunded` and `Disputed → Refunded`; rejects all transitions from `Refunded`.
- **`refunded` event** — emits `("INVOICE", "refunded")` with `(invoice_id, client, amount)` on every successful refund.
- **Tests** — covers happy paths for both `Cancelled` and `Disputed` sources, status assertion post-refund, wrong-status rejection, double-refund prevention, and event verification.

## Motivation

Without this function, funds deposited into escrow for a cancelled (post-funding) or disputed invoice were permanently locked in the contract with no recovery mechanism for the client.

## Testing

All new behaviour is covered by integration tests in `contracts/invoice/tests/test_refund_client.rs`.
