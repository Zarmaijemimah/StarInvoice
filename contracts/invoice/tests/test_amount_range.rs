#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, String};
    use star_invoice::{InvoiceContract, InvoiceContractClient};

    /// Creates a minimal invoice with the given amount. Token address is a dummy
    /// (no funding needed for these read-only query tests).
    fn create_invoice(
        env: &Env,
        client: &InvoiceContractClient,
        freelancer: &Address,
        payer: &Address,
        amount: i128,
    ) -> u64 {
        let token = Address::generate(env);
        let title = String::from_str(env, "Test");
        let desc = String::from_str(env, "Test invoice");
        client.create_invoice(freelancer, payer, &amount, &token, &9_999_999_999, &title, &desc)
    }

    fn setup() -> (Env, InvoiceContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        (env, client, freelancer, payer)
    }

    #[test]
    fn test_amount_range_returns_matching_invoices() {
        let (env, client, freelancer, payer) = setup();

        create_invoice(&env, &client, &freelancer, &payer, 100);
        create_invoice(&env, &client, &freelancer, &payer, 500);
        create_invoice(&env, &client, &freelancer, &payer, 1000);
        create_invoice(&env, &client, &freelancer, &payer, 2000);

        let results = client.get_invoices_by_amount_range(&200, &1000);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|inv| inv.amount == 500));
        assert!(results.iter().any(|inv| inv.amount == 1000));
    }

    #[test]
    fn test_amount_range_inclusive_bounds() {
        let (env, client, freelancer, payer) = setup();

        create_invoice(&env, &client, &freelancer, &payer, 100);
        create_invoice(&env, &client, &freelancer, &payer, 200);
        create_invoice(&env, &client, &freelancer, &payer, 300);

        // Both boundary values must be included
        let results = client.get_invoices_by_amount_range(&100, &300);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_amount_range_no_matches_returns_empty() {
        let (env, client, freelancer, payer) = setup();

        create_invoice(&env, &client, &freelancer, &payer, 50);
        create_invoice(&env, &client, &freelancer, &payer, 75);

        let results = client.get_invoices_by_amount_range(&1000, &5000);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_amount_range_exact_match() {
        let (env, client, freelancer, payer) = setup();

        create_invoice(&env, &client, &freelancer, &payer, 999);
        create_invoice(&env, &client, &freelancer, &payer, 1000);
        create_invoice(&env, &client, &freelancer, &payer, 1001);

        let results = client.get_invoices_by_amount_range(&1000, &1000);
        assert_eq!(results.len(), 1);
        assert_eq!(results.get(0).unwrap().amount, 1000);
    }

    #[test]
    fn test_amount_range_empty_contract() {
        let (env, client, _freelancer, _payer) = setup();

        let results = client.get_invoices_by_amount_range(&0, &9_999_999);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_amount_range_all_invoices_match() {
        let (env, client, freelancer, payer) = setup();

        for amount in [100i128, 200, 300, 400, 500] {
            create_invoice(&env, &client, &freelancer, &payer, amount);
        }

        let results = client.get_invoices_by_amount_range(&1, &10_000);
        assert_eq!(results.len(), 5);
    }
}
