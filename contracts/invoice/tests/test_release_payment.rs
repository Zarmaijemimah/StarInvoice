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

    fn create_approved_invoice(
        env: &Env,
        c: &InvoiceContractClient,
        freelancer: &Address,
        client: &Address,
        token: &Address,
        amount: i128,
    ) -> u64 {
        let id = c.create_invoice(
            freelancer,
            client,
            &amount,
            token,
            &9999999999,
            &String::from_str(env, "Test"),
            &String::from_str(env, "Test"),
            &String::from_str(env, ""),
        );
        c.fund_invoice(&id, token);
        c.mark_delivered(&id);
        c.approve_payment(&id);
        id
    }

    #[test]
    fn test_release_payment_transfers_funds_to_freelancer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let token_client = token::Client::new(&env, &token);

        let id = create_approved_invoice(&env, &c, &freelancer, &client, &token, amount);

        assert_eq!(token_client.balance(&contract_id), amount);
        assert_eq!(token_client.balance(&freelancer), 0);

        c.release_payment(&id);

        assert_eq!(token_client.balance(&freelancer), amount);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    fn test_release_payment_sets_status_to_completed() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let id = create_approved_invoice(&env, &c, &freelancer, &client, &token, amount);
        c.release_payment(&id);

        let invoice = c.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Completed);
    }

    #[test]
    fn test_release_payment_on_non_approved_invoice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let id = c.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token,
            &9999999999,
            &String::from_str(&env, "Test"),
            &String::from_str(&env, "Test"),
            &String::from_str(&env, ""),
        );
        c.fund_invoice(&id, &token);

        let result = c.try_release_payment(&id);
        assert_eq!(result, Err(Ok(ContractError::InvalidInvoiceStatus)));
    }

    #[test]
    fn test_release_payment_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let id = create_approved_invoice(&env, &c, &freelancer, &client, &token, amount);
        c.release_payment(&id);

        let result = c.try_release_payment(&id);
        assert_eq!(result, Err(Ok(ContractError::InvalidInvoiceStatus)));
    }
}
