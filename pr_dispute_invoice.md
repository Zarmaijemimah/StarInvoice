## Description
This PR implements the ability for either party (freelancer or client) to raise a dispute regarding an invoice.

### Acceptance Criteria Done:
- Added `Disputed` state to the `InvoiceStatus` enum.
- Added `pub fn dispute_invoice(env: Env, invoice_id: u64, caller: Address)` to `lib.rs`.
- Made sure disputes are only allowed from the `Funded` or `Delivered` status.
- Required explicit authorization from either the freelancer or the client.
- Automatically updates invoice status to `Disputed`.
- Emits the `invoice_disputed` event (`("INVOICE", "disputed")`).
- Embedded tests for both the happy path and invalid status exceptions.
