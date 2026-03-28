# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned Features
- Implement `fund_invoice` - escrow funding via token transfer
- Implement `mark_delivered` - freelancer signals work completion
- Implement `approve_payment` - client approves delivered work
- Implement `release_payment` - release escrowed funds to freelancer
- Add `Disputed` and `Cancelled` status variants to `InvoiceStatus`
- Add `token` field to `Invoice` for multi-token support
- Add `deadline` field to `Invoice` for time-bound agreements
- Add event emitters for all state transitions
- Add `cancel_invoice` function
- Add `dispute_invoice` function
- Add `refund_client` function for cancelled/disputed invoices
- Add invoice indexing by freelancer and client addresses
- Add comprehensive test coverage for all escrow flows
- Add state machine diagram to documentation

## [0.1.0] - Initial Release

### Added
- Initial project structure with Soroban smart contract setup
- `create_invoice` function - creates invoices with unique IDs
- `Invoice` struct with core fields: `id`, `freelancer`, `client`, `amount`, `description`, `status`
- `InvoiceStatus` enum with states: `Pending`, `Funded`, `Delivered`, `Approved`, `Completed`
- Storage layer with `next_invoice_id`, `save_invoice`, and `get_invoice` helpers
- Event emission for invoice creation
- Basic test coverage for invoice creation
- Project documentation: README, CONTRIBUTING, and ISSUES files
- Development tooling: Makefile, rustfmt, clippy configuration
