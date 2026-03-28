# Pull Request Descriptions

Below are the descriptions for all three issues you can copy and paste directly into GitHub!

---

## 1. Client Refund Event Emitter
**PR Title:** `feat: add client refund event emitter`

**Description Body (Copy below this line):**

## Description
This PR adds the `invoice_refunded` event emitter to the `events.rs` file.

Adds the `pub fn invoice_refunded(env: &Env, invoice_id: u64, client: &Address, amount: i128)` function which publishes under the `("INVOICE", "refunded")` topic.

Closes #65

---

## 2. Release Payment Event Emitter
**PR Title:** `feat: add release payment event emitter`

**Description Body (Copy below this line):**

## Description
This PR implements the `invoice_released` event emitter for the `release_payment` state transition.

### Acceptance Criteria Done:
- Added `pub fn invoice_released(env: &Env, invoice_id: u64, freelancer: &Address, amount: i128)` to `events.rs`.
- Published with topic `("INVOICE", "released")`.
- Updated `release_payment` in `lib.rs` to fetch the invoice and call the event before checking for token transfer implementation.

Closes #7

---

## 3. Dispute Invoice Functionality
**PR Title:** `feat: add dispute invoice functionality`

**Description Body (Copy below this line):**

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

Closes #5
