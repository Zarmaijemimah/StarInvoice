#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{InvoiceContract, InvoiceContractClient, InvoiceStatus, validate_transition};

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

    fn create_pending_invoice(
        env: &Env,
        contract_client: &InvoiceContractClient,
        freelancer: &Address,
        client: &Address,
        token_address: &Address,
        amount: i128,
    ) -> u64 {
        let title = String::from_str(env, "Test");
        let description = String::from_str(env, "Test invoice");
        let metadata_uri = String::from_str(env, "");
        contract_client.create_invoice(freelancer, client, &amount, token_address, &9999999999, &title, &description, &metadata_uri)
    }

    #[test]
    fn test_cancel_invoice_by_freelancer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_pending_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        contract_client.cancel_invoice(&invoice_id, &freelancer);

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Cancelled);
    }

    #[test]
    fn test_cancel_invoice_by_client() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_pending_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        contract_client.cancel_invoice(&invoice_id, &client);

        let invoice = contract_client.get_invoice(&invoice_id);
        assert_eq!(invoice.status, InvoiceStatus::Cancelled);
    }

    #[test]
    fn test_cancel_funded_invoice_returns_error() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_pending_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        contract_client.fund_invoice(&invoice_id, &token_address);

        let result = contract_client.try_cancel_invoice(&invoice_id, &freelancer);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_invoice_stranger_returns_error() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_pending_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        let stranger = Address::generate(&env);
        let result = contract_client.try_cancel_invoice(&invoice_id, &stranger);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_invoice_wrong_status() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_pending_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        contract_client.cancel_invoice(&invoice_id, &freelancer);

        let result = contract_client.try_cancel_invoice(&invoice_id, &freelancer);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_invoice_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address, amount) = setup(&env);
        let invoice_id = create_pending_invoice(&env, &contract_client, &freelancer, &client, &token_address, amount);

        let unrelated = Address::generate(&env);
        let result = contract_client.try_cancel_invoice(&invoice_id, &unrelated);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_transitions() {
        assert!(validate_transition(&InvoiceStatus::Pending, &InvoiceStatus::Funded));
        assert!(validate_transition(&InvoiceStatus::Pending, &InvoiceStatus::Cancelled));
        assert!(validate_transition(&InvoiceStatus::Funded, &InvoiceStatus::Delivered));
        assert!(validate_transition(&InvoiceStatus::Funded, &InvoiceStatus::Disputed));
        assert!(validate_transition(&InvoiceStatus::Delivered, &InvoiceStatus::Approved));
        assert!(validate_transition(&InvoiceStatus::Delivered, &InvoiceStatus::Disputed));
        assert!(validate_transition(&InvoiceStatus::Approved, &InvoiceStatus::Completed));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!validate_transition(&InvoiceStatus::Pending, &InvoiceStatus::Delivered));
        assert!(!validate_transition(&InvoiceStatus::Pending, &InvoiceStatus::Approved));
        assert!(!validate_transition(&InvoiceStatus::Pending, &InvoiceStatus::Completed));
        assert!(!validate_transition(&InvoiceStatus::Funded, &InvoiceStatus::Cancelled));
        assert!(!validate_transition(&InvoiceStatus::Funded, &InvoiceStatus::Approved));
        assert!(!validate_transition(&InvoiceStatus::Delivered, &InvoiceStatus::Funded));
        assert!(!validate_transition(&InvoiceStatus::Cancelled, &InvoiceStatus::Pending));
        assert!(!validate_transition(&InvoiceStatus::Completed, &InvoiceStatus::Approved));
    }
}
