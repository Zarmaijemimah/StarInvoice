# Requirements Document

## Introduction

This feature adds a `refund_client` function to the StarInvoice Soroban smart contract. When an invoice is cancelled after funding (i.e., the escrow already holds tokens), or when a dispute is resolved in the client's favour, the escrowed funds must be returnable to the client. Currently the contract has no path to return funds to the client; this feature closes that gap.

## Glossary

- **Contract**: The `InvoiceContract` Soroban smart contract.
- **Client**: The address that funded the invoice and is owed a refund.
- **Freelancer**: The address that created the invoice.
- **Escrow**: Funds held by the Contract on behalf of the parties.
- **Invoice**: The on-chain data structure tracking the escrow lifecycle.
- **InvoiceStatus**: The enum representing the current lifecycle state of an Invoice (`Pending`, `Funded`, `Delivered`, `Disputed`, `Approved`, `Completed`, `Cancelled`, `Refunded`).
- **Refunded**: A new `InvoiceStatus` variant indicating that escrowed funds have been returned to the Client.
- **Token**: The Stellar asset contract used for payment.
- **invoice_id**: The unique `u64` identifier for an Invoice.

## Requirements

### Requirement 1: Add Refunded Status Variant

**User Story:** As a contract developer, I want a dedicated `Refunded` status, so that the lifecycle of a refunded invoice is clearly distinguishable from `Completed` (freelancer paid) and `Cancelled` (no funds ever held).

#### Acceptance Criteria

1. THE Contract SHALL expose a `Refunded` variant in the `InvoiceStatus` enum.
2. THE Contract SHALL treat `Refunded` as a terminal state: no further status transitions are permitted from `Refunded`.

---

### Requirement 2: Refund Client Function

**User Story:** As a client, I want to call `refund_client(invoice_id)` to recover escrowed funds when an invoice is cancelled after funding or when a dispute is resolved in my favour, so that my tokens are not permanently locked in the contract.

#### Acceptance Criteria

1. THE Contract SHALL expose a public function `refund_client(env: Env, invoice_id: u64)`.
2. WHEN `refund_client` is called and the Invoice status is `Cancelled` or `Disputed`, THE Contract SHALL transfer the full escrowed amount from the Contract to the Client address recorded on the Invoice.
3. WHEN `refund_client` is called and the Invoice status is neither `Cancelled` nor `Disputed`, THE Contract SHALL return `ContractError::InvalidInvoiceStatus` without modifying any state.
4. WHEN `refund_client` is called and the Invoice does not exist, THE Contract SHALL return `ContractError::InvoiceNotFound`.
5. AFTER a successful `refund_client` call, THE Contract SHALL set the Invoice status to `Refunded`.
6. AFTER a successful `refund_client` call, THE Contract SHALL hold zero tokens for that Invoice (i.e., the escrowed amount has been fully transferred to the Client).

---

### Requirement 3: Refunded Event Emission

**User Story:** As an off-chain observer, I want a `refunded` event emitted on every successful refund, so that I can index and react to refund activity without polling contract state.

#### Acceptance Criteria

1. WHEN `refund_client` succeeds, THE Contract SHALL emit an event with topic `("INVOICE", "refunded")` and data `(invoice_id, client, amount)`.
2. IF `refund_client` fails for any reason, THE Contract SHALL NOT emit a `refunded` event.

---

### Requirement 4: State Transition Validation

**User Story:** As a contract developer, I want the `validate_transition` function to recognise the new `Refunded` terminal state, so that the transition table remains the single source of truth for all lifecycle changes.

#### Acceptance Criteria

1. THE Contract SHALL permit the transition `Cancelled → Refunded` in `validate_transition`.
2. THE Contract SHALL permit the transition `Disputed → Refunded` in `validate_transition`.
3. THE Contract SHALL reject any transition originating from `Refunded` in `validate_transition`.

---

### Requirement 5: Idempotency and Double-Refund Prevention

**User Story:** As a contract developer, I want calling `refund_client` twice on the same invoice to fail, so that a client cannot drain the contract by replaying the call.

#### Acceptance Criteria

1. WHEN `refund_client` is called on an Invoice whose status is already `Refunded`, THE Contract SHALL return `ContractError::InvalidInvoiceStatus`.
2. THE Contract SHALL NOT transfer any tokens when returning `ContractError::InvalidInvoiceStatus`.

---

### Requirement 6: Test Coverage

**User Story:** As a developer, I want comprehensive tests for `refund_client`, so that regressions are caught automatically.

#### Acceptance Criteria

1. THE test suite SHALL include a test verifying that `refund_client` transfers the full escrowed amount to the Client when the Invoice is `Cancelled` (post-funding).
2. THE test suite SHALL include a test verifying that `refund_client` transfers the full escrowed amount to the Client when the Invoice is `Disputed`.
3. THE test suite SHALL include a test verifying that `refund_client` sets the Invoice status to `Refunded` after a successful call.
4. THE test suite SHALL include a test verifying that `refund_client` returns `ContractError::InvalidInvoiceStatus` when the Invoice is in any status other than `Cancelled` or `Disputed` (e.g., `Pending`, `Funded`, `Approved`, `Completed`).
5. THE test suite SHALL include a test verifying that calling `refund_client` twice on the same Invoice returns `ContractError::InvalidInvoiceStatus` on the second call.
6. THE test suite SHALL include a test verifying that the `refunded` event is emitted with the correct `(invoice_id, client, amount)` data on a successful refund.
