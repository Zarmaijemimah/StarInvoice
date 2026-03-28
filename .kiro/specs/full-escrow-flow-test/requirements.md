# Requirements Document

## Introduction

Add a single integration test that walks through the complete escrow lifecycle for the Soroban invoice contract. The test covers all five state transitions in sequence: `create_invoice` → `fund_invoice` → `mark_delivered` → `approve_payment` → `release_payment`, asserting invoice status at each step and verifying final token balances.

## Glossary

- **InvoiceContract**: The Soroban smart contract under test, deployed via `env.register_contract`.
- **InvoiceContractClient**: The auto-generated test client used to call contract methods.
- **Escrow_Flow**: The full lifecycle of an invoice from creation through payment release.
- **Invoice_Status**: The `InvoiceStatus` enum value stored on-chain (`Pending`, `Funded`, `Delivered`, `Approved`, `Completed`).
- **Token_Client**: A `token::Client` instance used to query on-chain token balances.
- **Freelancer**: The address that creates the invoice and receives payment upon completion.
- **Client**: The address that funds the invoice and approves delivery.

## Requirements

### Requirement 1: Single Integration Test Function

**User Story:** As a developer, I want a single test function that exercises the entire escrow flow, so that I can verify all five steps work together end-to-end without needing to trace across multiple test files.

#### Acceptance Criteria

1. THE Test_Suite SHALL contain exactly one test function named `test_full_escrow_flow` that covers all five contract calls in sequence.
2. WHEN the test runs, THE Test_Suite SHALL call `create_invoice`, `fund_invoice`, `mark_delivered`, `approve_payment`, and `release_payment` in that order within the same function body.

### Requirement 2: Status Assertions at Each Step

**User Story:** As a developer, I want the test to assert the invoice status after each state transition, so that I can pinpoint exactly which step fails if a regression is introduced.

#### Acceptance Criteria

1. WHEN `create_invoice` returns, THE Test_Suite SHALL assert that the invoice status equals `InvoiceStatus::Pending`.
2. WHEN `fund_invoice` returns, THE Test_Suite SHALL assert that the invoice status equals `InvoiceStatus::Funded`.
3. WHEN `mark_delivered` returns, THE Test_Suite SHALL assert that the invoice status equals `InvoiceStatus::Delivered`.
4. WHEN `approve_payment` returns, THE Test_Suite SHALL assert that the invoice status equals `InvoiceStatus::Approved`.
5. WHEN `release_payment` returns, THE Test_Suite SHALL assert that the invoice status equals `InvoiceStatus::Completed`.

### Requirement 3: Final Token Balance Assertions

**User Story:** As a developer, I want the test to assert token balances after `release_payment`, so that I can confirm funds move correctly from escrow to the freelancer.

#### Acceptance Criteria

1. WHEN `release_payment` returns, THE Test_Suite SHALL assert that the freelancer's token balance equals the original invoice amount.
2. WHEN `release_payment` returns, THE Test_Suite SHALL assert that the contract's token balance equals zero.
3. WHEN `fund_invoice` returns, THE Test_Suite SHALL assert that the contract's token balance equals the invoice amount (funds are held in escrow).
