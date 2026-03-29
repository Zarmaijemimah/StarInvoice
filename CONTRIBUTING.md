# Contributing to StarInvoice

Thanks for your interest in contributing. This project is designed to be built incrementally — most of the escrow logic is intentionally left unimplemented so contributors can own features end-to-end.

## Code of Conduct

This project adheres to the Contributor Covenant [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

1. Fork the repo and clone it locally
2. Install Rust and the Soroban CLI (see README)
3. Run `cargo test` to confirm the baseline passes
4. Pick an open issue and comment to claim it

## Open Issues (Good First Contributions)

Each TODO in the codebase maps to a GitHub issue:

| Issue | Function           | Description                                      |
|-------|--------------------|--------------------------------------------------|
| #1    | `fund_invoice`     | Implement escrow funding via token transfer      |
| #2    | `mark_delivered`   | Allow freelancer to signal work completion       |
| #3    | `approve_payment`  | Allow client to approve delivered work           |
| #4    | `release_payment`  | Release escrowed funds to freelancer             |
| #5    | `InvoiceStatus`    | Add Disputed and Cancelled status variants       |
| #6    | `Invoice`          | Add deadline and multi-token support fields      |
| #7    | `events.rs`        | Add event emitters for all state transitions     |

## Guidelines

- Keep changes focused — one issue per PR
- Add or update tests for any logic you implement
- Follow existing code style (run `cargo fmt` before committing)
- Add event emissions for any state-changing functions
- Update the README status table when a function is fully implemented

## Code Style

This project uses `rustfmt.toml` to enforce consistent formatting. All code must pass formatting checks before merging.

```bash
cargo fmt        # format (must pass before committing)
cargo clippy     # lint
cargo test       # test
cargo audit      # check dependencies for vulnerabilities
```

Ensure `cargo fmt` passes on all changes before opening a PR.

## Pull Request Process

1. Open a PR referencing the issue number (e.g. `Closes #1`)
2. Describe what you implemented and any design decisions
3. Ensure all tests pass
4. A maintainer will review and merge

## Questions?

Open a GitHub Discussion or comment on the relevant issue.
