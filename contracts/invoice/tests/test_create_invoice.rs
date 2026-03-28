#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{InvoiceContract, InvoiceContractClient, InvoiceStatus, ContractError};

    fn setup(env: &Env) -> (Address, Address, Address, i128) {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();

        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        let amount: i128 = 1000;

        (freelancer, client, token_address, amount)
    }

    #[test]
    fn test_create_invoice_success() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let description = String::from_str(&env, "Test invoice");

        let invoice_id = contract_client.create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &description).unwrap();

        let invoice = contract_client.get_invoice(&invoice_id).unwrap();
        assert_eq!(invoice.status, InvoiceStatus::Pending);
        assert_eq!(invoice.freelancer, freelancer);
        assert_eq!(invoice.client, client);
        assert_eq!(invoice.amount, amount);
    }

    #[test]
    fn test_create_invoice_description_at_max_length() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let description = String::from_str(&env, &"a".repeat(256));

        let result = contract_client.try_create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_invoice_description_exceeds_max_length() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let description = String::from_str(&env, &"a".repeat(257));

        let result = contract_client.try_create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &description);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::DescriptionTooLong);
    }

    #[test]
    fn test_create_invoice_empty_description() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let description = String::from_str(&env, "");

        let result = contract_client.try_create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_invoice_unique_ids() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let description = String::from_str(&env, "Unique ID test");

        let mut seen_ids = std::collections::HashSet::new();

        for expected_id in 0u64..10 {
            let invoice_id = contract_client.create_invoice(
                &freelancer,
                &client,
                &amount,
                &token_address,
                &9999999999,
                &description,
            ).unwrap();

            assert_eq!(invoice_id, expected_id, "ID should increment sequentially");
            assert!(seen_ids.insert(invoice_id), "Duplicate ID detected: {}", invoice_id);
        }

        assert_eq!(seen_ids.len(), 10);
    }
}
