#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{ContractError, InvoiceContract, InvoiceContractClient};

    fn setup(env: &Env) -> (Address, Address, Address, i128) {
        let token_admin = Address::generate(env);
        let token_address = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        token::StellarAssetClient::new(env, &token_address).mint(&Address::generate(env), &0);

        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        let amount: i128 = 1000;

        token::StellarAssetClient::new(env, &token_address).mint(&client, &(amount * 2));

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
        let id = client.create_invoice(
            freelancer,
            payer,
            &amount,
            token,
            &9999999999,
            &String::from_str(env, "Test"),
            &String::from_str(env, "Test"),
        );
        client.fund_invoice(&id, token);
        id
    }

    #[test]
    fn test_fund_invoice_already_funded() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let id = create_funded_invoice(&env, &c, &freelancer, &client, &token, amount);

        let result = c.try_fund_invoice(&id, &token);
        assert_eq!(
            result,
            Err(Ok(ContractError::InvalidInvoiceStatus)),
            "funding an already-Funded invoice must return InvalidInvoiceStatus"
        );
    }

    #[test]
    fn test_mark_delivered_on_pending_invoice() {
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
        );

        let result = c.try_mark_delivered(&id);
        assert_eq!(
            result,
            Err(Ok(ContractError::InvalidInvoiceStatus)),
            "mark_delivered on a Pending invoice must return InvalidInvoiceStatus"
        );
    }

    #[test]
    fn test_approve_payment_on_funded_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let id = create_funded_invoice(&env, &c, &freelancer, &client, &token, amount);

        let result = c.try_approve_payment(&id);
        assert_eq!(
            result,
            Err(Ok(ContractError::InvalidInvoiceStatus)),
            "approve_payment on a Funded invoice must return InvalidInvoiceStatus"
        );
    }

    #[test]
    fn test_release_payment_on_delivered_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        let id = create_funded_invoice(&env, &c, &freelancer, &client, &token, amount);
        c.mark_delivered(&id);

        let result = c.try_release_payment(&id);
        assert_eq!(
            result,
            Err(Ok(ContractError::InvalidInvoiceStatus)),
            "release_payment on a Delivered invoice must return InvalidInvoiceStatus"
        );
    }
}
