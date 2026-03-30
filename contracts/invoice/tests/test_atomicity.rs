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
    fn test_multiple_invoices_unique_ids() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let title = String::from_str(&env, "T");
        let description = String::from_str(&env, "Test invoice");
        let metadata_uri = String::from_str(&env, "");

        let mut ids = Vec::new();
        for _ in 0..10 {
            let id = contract_client.create_invoice(
                &freelancer, &client, &amount, &token_address, &9999999999, &title, &description, &metadata_uri,
            );
            ids.push(id);
        }

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "Invoice IDs must be unique");
            }
        }

        for (i, &id) in ids.iter().enumerate() {
            assert_eq!(id, i as u64, "Invoice IDs should be sequential starting from 0");
        }
    }
}
