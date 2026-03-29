#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{ContractError, InvoiceContract, InvoiceContractClient};

    fn setup(env: &Env) -> (Address, Address, Address) {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();

        let freelancer = Address::generate(env);
        let client = Address::generate(env);

        (freelancer, client, token_address)
    }

    #[test]
    fn test_create_invoice_zero_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Test");
        let description = String::from_str(&env, "Test invoice");

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &0, &token_address, &9999999999, &title, &description,
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
    }

    #[test]
    fn test_create_invoice_negative_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Test");
        let description = String::from_str(&env, "Test invoice");

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &-100, &token_address, &9999999999, &title, &description,
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
    }

    #[test]
    fn test_create_invoice_positive_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

        let freelancer = Address::generate(&env);
        let client = Address::generate(&env);
        let amount: i128 = 1;

        token_admin_client.mint(&client, &amount);

        let title = String::from_str(&env, "Test");
        let description = String::from_str(&env, "Test invoice");

        let id = contract_client.create_invoice(
            &freelancer, &client, &amount, &token_address, &9999999999, &title, &description,
        );
        assert_eq!(id, 0);
    }
}
