## Description
This PR adds the `invoice_refunded` event emitter to the `events.rs` file.

Adds the `pub fn invoice_refunded(env: &Env, invoice_id: u64, client: &Address, amount: i128)` function which publishes under the `("INVOICE", "refunded")` topic.

Closes #65
