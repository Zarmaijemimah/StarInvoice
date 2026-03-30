#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{ContractError, InvoiceContract, InvoiceContractClient, InvoiceStatus};

    fn setup(env: &Env) -> (Address, Address, Address, i128) {
        let token_admin = Address::generate(env);
        let token_address = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        let amount: i128 = 1000;
        token::StellarAssetClient::new(env, &token_address).mint(&client, &amount);
        (freelancer, client, token_address, amount)
    }

    #[test]
    fn test_fund_invoice_with_correct_token_succeeds() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let invoice_id = c.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token,
            &9999999999,
            &String::from_str(&env, "Test Invoice"),
            &String::from_str(&env, "Test Invoice"),
            &String::from_str(&env, ""),
        );

        let result = c.try_fund_invoice(&invoice_id, &token);
        assert!(result.is_ok());

        let invoice = c.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Funded);

        let token_client = token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&contract_id), amount);
    }

    #[test]
    fn test_fund_invoice_with_incorrect_token_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let token_admin_2 = Address::generate(&env);
        let wrong_token = env
            .register_stellar_asset_contract_v2(token_admin_2)
            .address();

        let invoice_id = c.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token,
            &9999999999,
            &String::from_str(&env, "Test Invoice"),
            &String::from_str(&env, "Test Invoice"),
            &String::from_str(&env, ""),
        );

        let result = c.try_fund_invoice(&invoice_id, &wrong_token);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().unwrap(),
            ContractError::TokenMismatch,
            "Expected TokenMismatch error when funding with incorrect token"
        );

        let invoice = c.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Pending);

        let token_client = token::Client::new(&env, &token);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    fn test_fund_invoice_validation_happens_before_state_mutation() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let token_admin_2 = Address::generate(&env);
        let wrong_token = env
            .register_stellar_asset_contract_v2(token_admin_2)
            .address();

        let invoice_id = c.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token,
            &9999999999,
            &String::from_str(&env, "Test Invoice"),
            &String::from_str(&env, "Test Invoice"),
            &String::from_str(&env, ""),
        );

        let result = c.try_fund_invoice(&invoice_id, &wrong_token);
        assert!(result.is_err());

        let result = c.try_fund_invoice(&invoice_id, &token);
        assert!(result.is_ok());

        let invoice = c.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Funded);
    }
}
