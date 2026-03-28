use soroban_sdk::{symbol_short, Address, Env};

/// Emits an event when a new invoice is created.
///
/// Topic: `("INVOICE", "created")`
/// Data:  `(invoice_id, freelancer, client, amount)`
pub fn invoice_created(
    env: &Env,
    invoice_id: u64,
    freelancer: &Address,
    client: &Address,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("created")),
        (invoice_id, freelancer.clone(), client.clone(), amount),
    );
}

/// Emits an event when an invoice is funded by the client.
///
/// Topic: `("INVOICE", "funded")`
/// Data:  `(invoice_id, client, amount)`
pub fn invoice_funded(env: &Env, invoice_id: u64, client: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("funded")),
        (invoice_id, client.clone(), amount),
    );
}

/// Emits an event when a freelancer marks work as delivered.
///
/// Topic: `("INVOICE", "delivered")`
/// Data:  `(invoice_id, freelancer)`
pub fn mark_delivered(env: &Env, invoice_id: u64, freelancer: &Address) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("delivered")),
        (invoice_id, freelancer.clone()),
    );
}

/// Emits an event when a client approves payment for a delivered invoice.
///
/// Topic: `("INVOICE", "approved")`
/// Data:  `(invoice_id, client)`
pub fn invoice_approved(env: &Env, invoice_id: u64, client: &Address) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("approved")),
        (invoice_id, client.clone()),
    );
}

/// Emits an event when an invoice is cancelled.
///
/// Topic: `("INVOICE", "cancelled")`
/// Data:  `(invoice_id, cancelled_by)`
pub fn invoice_cancelled(env: &Env, invoice_id: u64, cancelled_by: &Address) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("cancelled")),
        (invoice_id, cancelled_by.clone()),
    );
}

/// Emits an event when a client disputes a delivered or funded invoice.
///
/// Topic: `("INVOICE", "disputed")`
/// Data:  `(invoice_id, client)`
pub fn invoice_disputed(env: &Env, invoice_id: u64, client: &Address) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("disputed")),
        (invoice_id, client.clone()),
    );
}

/// Emits an event when escrowed funds are released to the freelancer.
///
/// Topic: `("INVOICE", "released")`
/// Data:  `(invoice_id, freelancer, amount)`
pub fn release_payment(env: &Env, invoice_id: u64, freelancer: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("released")),
        (invoice_id, freelancer.clone(), amount),
    );
}

// TODO: Add event emitters for each state transition:
// - mark_delivered  -> emit "INVOICE delivered" | data: (invoice_id, freelancer)
// See: https://github.com/your-org/StarInvoice/issues/7

/// Emits an event when escrowed funds are released to the freelancer.
///
/// Topic: `("INVOICE", "released")`
/// Data:  `(invoice_id, freelancer, amount)`
pub fn invoice_released(env: &Env, invoice_id: u64, freelancer: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("released")),
        (invoice_id, freelancer.clone(), amount),
    );
}

/// Emits an event when the contract admin is initialized.
///
/// Topic: `("ADMIN", "initialized")`
/// Data:  `(admin_address)`
pub fn admin_initialized(env: &Env, admin: &Address) {
    env.events().publish(
        (symbol_short!("ADMIN"), symbol_short!("init")),
        admin.clone(),
    );
}

/// Emits an event when the admin address is changed.
///
/// Topic: `("ADMIN", "changed")`
/// Data:  `(old_admin, new_admin)`
pub fn admin_changed(env: &Env, old_admin: &Address, new_admin: &Address) {
    env.events().publish(
        (symbol_short!("ADMIN"), symbol_short!("changed")),
        (old_admin.clone(), new_admin.clone()),
    );
}

/// Emits an event when a dispute is resolved by the admin.
///
/// Topic: `("DISPUTE", "resolved")`
/// Data:  `(invoice_id, winner)`
pub fn dispute_resolved(env: &Env, invoice_id: u64, winner: &Address) {
    env.events().publish(
        (symbol_short!("DISPUTE"), symbol_short!("resolved")),
        (invoice_id, winner.clone()),
    );
}

/// Emits an event when an invoice is refunded to the client.
///
/// Topic: `("INVOICE", "refunded")`
/// Data:  `(invoice_id, client, amount)`
pub fn invoice_refunded(env: &Env, invoice_id: u64, client: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("refunded")),
        (invoice_id, client.clone(), amount),
    );
}

/// Emits an event when an invoice is disputed.
///
/// Topic: `("INVOICE", "disputed")`
/// Data:  `(invoice_id, caller)`
pub fn invoice_disputed(env: &Env, invoice_id: u64, caller: &Address) {
    env.events().publish(
        (symbol_short!("INVOICE"), symbol_short!("disputed")),
        (invoice_id, caller.clone()),
    );
}

