#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{InvoiceContract, InvoiceContractClient};

    fn setup(env: &Env) -> (Address, Address, Address, i128) {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(env, &token_address);

        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        let amount: i128 = 1000;

        token_admin_client.mint(&client, &amount);

        (freelancer, client, token_address, amount)
    }

    #[test]
    fn test_get_invoices_by_freelancer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "Test invoice");
        let metadata_uri = String::from_str(&env, "");

        let id1 = contract_client.create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &title, &description, &metadata_uri);
        let id2 = contract_client.create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &title, &description, &metadata_uri);

        let invoices = contract_client.get_invoices_by_freelancer(&freelancer);
        assert_eq!(invoices.len(), 2);
        assert_eq!(invoices.get(0).unwrap(), id1);
        assert_eq!(invoices.get(1).unwrap(), id2);
    }


    #[test]
    fn test_get_invoices_by_client() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "Test invoice");
        let metadata_uri = String::from_str(&env, "");

        let id1 = contract_client.create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &title, &description, &metadata_uri);
        let id2 = contract_client.create_invoice(&freelancer, &client, &amount, &token_address, &9999999999, &title, &description, &metadata_uri);

        let invoices = contract_client.get_invoices_by_client(&client);
        assert_eq!(invoices.len(), 2);
        assert_eq!(invoices.get(0).unwrap(), id1);
        assert_eq!(invoices.get(1).unwrap(), id2);
    }

    #[test]
    fn test_get_invoices_by_freelancer_multiple_clients() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

        let freelancer = Address::generate(&env);
        let client1 = Address::generate(&env);
        let client2 = Address::generate(&env);
        let amount: i128 = 1000;

        token_admin_client.mint(&client1, &amount);
        token_admin_client.mint(&client2, &amount);

        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "Test invoice");
        let metadata_uri = String::from_str(&env, "");

        let id1 = contract_client.create_invoice(&freelancer, &client1, &amount, &token_address, &9999999999, &title, &description, &metadata_uri);
        let id2 = contract_client.create_invoice(&freelancer, &client2, &amount, &token_address, &9999999999, &title, &description, &metadata_uri);

        let freelancer_invoices = contract_client.get_invoices_by_freelancer(&freelancer);
        assert_eq!(freelancer_invoices.len(), 2);

        let client1_invoices = contract_client.get_invoices_by_client(&client1);
        assert_eq!(client1_invoices.len(), 1);
        assert_eq!(client1_invoices.get(0).unwrap(), id1);

        let client2_invoices = contract_client.get_invoices_by_client(&client2);
        assert_eq!(client2_invoices.len(), 1);
        assert_eq!(client2_invoices.get(0).unwrap(), id2);
    }
}
