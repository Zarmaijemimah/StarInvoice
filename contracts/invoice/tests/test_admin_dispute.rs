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
        let amount: i128 = 100;
        token::StellarAssetClient::new(env, &token_address).mint(&client, &amount);
        (freelancer, client, token_address, amount)
    }

    #[test]
    fn test_resolve_dispute_by_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        contract_client.initialize(&admin);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = contract_client.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token_address,
            &1000,
            &String::from_str(&env, "Test"),
            &String::from_str(&env, "Desc"),
        );
        contract_client.fund_invoice(&invoice_id, &token_address);
        contract_client.dispute_invoice(&invoice_id);
        contract_client.resolve_dispute(&invoice_id, &freelancer);

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Completed);
    }

    #[test]
    fn test_resolve_dispute_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        contract_client.initialize(&admin);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = contract_client.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token_address,
            &1000,
            &String::from_str(&env, "Test"),
            &String::from_str(&env, "Desc"),
        );
        contract_client.fund_invoice(&invoice_id, &token_address);
        contract_client.dispute_invoice(&invoice_id);

        // Remove all auths so resolve_dispute fails
        env.set_auths(&[]);
        let result = contract_client.try_resolve_dispute(&invoice_id, &freelancer);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_dispute_invalid_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        contract_client.initialize(&admin);

        let result = contract_client.try_resolve_dispute(&9999, &admin);
        assert_eq!(result, Err(Ok(ContractError::InvoiceNotFound)));
    }

    #[test]
    fn test_set_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        contract_client.initialize(&admin1);
        contract_client.set_admin(&admin2);
    }
}
