#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, String};
    use star_invoice::{ContractError, InvoiceContract, InvoiceContractClient, InvoiceStatus};

    fn setup(env: &Env) -> (Address, Address, Address) {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        (freelancer, client, token_address)
    }

    #[test]
    fn test_create_invoice_at_max_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Max Amount Invoice");
        let description = String::from_str(&env, "Invoice at the maximum allowed amount");
        let metadata_uri = String::from_str(&env, "");
        let max_amount: i128 = 10_000_000_000_000;

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &max_amount, &token_address, &9999999999, &title, &description, &metadata_uri,
        );
        assert!(result.is_ok(), "Invoice creation should succeed at MAX_INVOICE_AMOUNT");
        let invoice_id = result.unwrap().unwrap();
        assert_eq!(invoice_id, 0, "First invoice should have ID 0");

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.amount, max_amount);
        assert_eq!(invoice.freelancer, freelancer);
        assert_eq!(invoice.client, client);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }

    #[test]
    fn test_create_invoice_exceeds_max_amount_by_one() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Over Max Invoice");
        let description = String::from_str(&env, "Invoice exceeding maximum amount");
        let metadata_uri = String::from_str(&env, "");
        let over_max_amount: i128 = 10_000_000_000_001;

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &over_max_amount, &token_address, &9999999999, &title, &description, &metadata_uri,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::AmountExceedsMaximum);
    }

    #[test]
    fn test_create_invoice_exceeds_max_amount_significantly() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Way Over Max Invoice");
        let description = String::from_str(&env, "Invoice far exceeding maximum amount");
        let metadata_uri = String::from_str(&env, "");
        let way_over_max: i128 = 100_000_000_000_000;

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &way_over_max, &token_address, &9999999999, &title, &description, &metadata_uri,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), ContractError::AmountExceedsMaximum);
    }

    #[test]
    fn test_create_invoice_normal_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Normal Invoice");
        let description = String::from_str(&env, "Invoice with normal amount");
        let metadata_uri = String::from_str(&env, "");
        let normal_amount: i128 = 1000;

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &normal_amount, &token_address, &9999999999, &title, &description, &metadata_uri,
        );
        assert!(result.is_ok());
        let invoice_id = result.unwrap().unwrap();

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.amount, normal_amount);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }

    #[test]
    fn test_create_invoice_large_but_valid_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Large Invoice");
        let description = String::from_str(&env, "Invoice with large but valid amount");
        let metadata_uri = String::from_str(&env, "");
        let large_amount: i128 = 5_000_000_000_000;

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &large_amount, &token_address, &9999999999, &title, &description, &metadata_uri,
        );
        assert!(result.is_ok());
        let invoice_id = result.unwrap().unwrap();

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.amount, large_amount);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }

    #[test]
    fn test_max_amount_validation_before_state_change() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);

        let id1 = contract_client.create_invoice(
            &freelancer, &client, &1000, &token_address, &9999999999,
            &String::from_str(&env, "First Invoice"),
            &String::from_str(&env, "First"),
            &String::from_str(&env, ""),
        );
        assert_eq!(id1, 0);

        let result2 = contract_client.try_create_invoice(
            &freelancer, &client, &100_000_000_000_000, &token_address, &9999999999,
            &String::from_str(&env, "Over Limit Invoice"),
            &String::from_str(&env, "This should fail"),
            &String::from_str(&env, ""),
        );
        assert!(result2.is_err());

        let id3 = contract_client.create_invoice(
            &freelancer, &client, &2000, &token_address, &9999999999,
            &String::from_str(&env, "Third Invoice"),
            &String::from_str(&env, "Third"),
            &String::from_str(&env, ""),
        );
        assert_eq!(id3, 1, "Third invoice should have ID 1, proving failed attempt did not increment state");
    }

    #[test]
    fn test_create_invoice_minimum_valid_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Minimum Invoice");
        let description = String::from_str(&env, "Minimum valid amount");
        let metadata_uri = String::from_str(&env, "");

        let result = contract_client.try_create_invoice(
            &freelancer, &client, &1, &token_address, &9999999999, &title, &description, &metadata_uri,
        );
        assert!(result.is_ok());
        let invoice_id = result.unwrap().unwrap();

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.amount, 1);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }
}
