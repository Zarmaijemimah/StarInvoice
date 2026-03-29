#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{InvoiceContract, InvoiceContractClient, InvoiceStatus};

    fn setup(env: &Env) -> (Address, Address, Address, i128) {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        token::StellarAssetClient::new(env, &token_address).mint(&Address::generate(env), &0);

        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        let amount: i128 = 1000;
        token::StellarAssetClient::new(env, &token_address).mint(&client, &amount);

        (freelancer, client, token_address, amount)
    }

    fn create_funded_invoice(
        env: &Env,
        client: &InvoiceContractClient,
        freelancer: &Address,
        payer: &Address,
        token: &Address,
        amount: i128,
    ) -> u64 {
        let title = String::from_str(env, "Test");
        let description = String::from_str(env, "Test invoice");
        let id = client.create_invoice(freelancer, payer, &amount, token, &9999999999, &title, &description);
        client.fund_invoice(&id, token);
        id
    }

    #[test]
    fn test_dispute_funded_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_funded_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        contract_client.dispute_invoice(&invoice_id);

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Disputed);
    }

    #[test]
    fn test_dispute_delivered_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_funded_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);
        contract_client.mark_delivered(&invoice_id);

        contract_client.dispute_invoice(&invoice_id);

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Disputed);
    }

    #[test]
    fn test_dispute_pending_invoice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let title = String::from_str(&env, "Test");
        let description = String::from_str(&env, "Test invoice");
        let invoice_id = contract_client.create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &title, &description);

        let result = contract_client.try_dispute_invoice(&invoice_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_approved_invoice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_funded_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);
        contract_client.mark_delivered(&invoice_id);
        contract_client.approve_payment(&invoice_id);

        let result = contract_client.try_dispute_invoice(&invoice_id);
        assert!(result.is_err());
    }
}
