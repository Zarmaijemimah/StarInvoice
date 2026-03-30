#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env, String};
    use star_invoice::{ContractError, InvoiceContract, InvoiceContractClient};

    fn setup(env: &Env) -> (Address, Address, Address, i128) {
        let token_admin = Address::generate(env);
        let token_address = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let freelancer = Address::generate(env);
        let client = Address::generate(env);
        let amount: i128 = 1000;
        token::StellarAssetClient::new(env, &token_address).mint(&client, &amount);
        (freelancer, client, token_address, amount)
    }

    #[test]
    fn test_fund_invoice_rejects_past_deadline() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        // Set ledger time to 1000 so a deadline of 1 is already in the past
        env.ledger().set_timestamp(1000);

        let past_deadline: u64 = 1;
        let invoice_id = c.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token,
            &past_deadline,
            &String::from_str(&env, "Expired Invoice"),
            &String::from_str(&env, "This invoice has a past deadline"),
        );

        let result = c.try_fund_invoice(&invoice_id, &token);
        assert_eq!(result, Err(Ok(ContractError::InvoiceExpired)));
    }

    #[test]
    fn test_fund_invoice_accepts_future_deadline() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let c = InvoiceContractClient::new(&env, &contract_id);
        let (freelancer, client, token, amount) = setup(&env);

        env.ledger().set_timestamp(1000);

        let future_deadline: u64 = 9_999_999_999;
        let invoice_id = c.create_invoice(
            &freelancer,
            &client,
            &amount,
            &token,
            &future_deadline,
            &String::from_str(&env, "Valid Invoice"),
            &String::from_str(&env, "This invoice has a future deadline"),
        );

        let result = c.try_fund_invoice(&invoice_id, &token);
        assert!(result.is_ok());
    }
}
