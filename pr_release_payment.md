## Description
This PR implements the `invoice_released` event emitter for the `release_payment` state transition.

### Acceptance Criteria Done:
- Added `pub fn invoice_released(env: &Env, invoice_id: u64, freelancer: &Address, amount: i128)` to `events.rs`.
- Published with topic `("INVOICE", "released")`.
- Updated `release_payment` in `lib.rs` to fetch the invoice and call the event before checking for token transfer implementation.
