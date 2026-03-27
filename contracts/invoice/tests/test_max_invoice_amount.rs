#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
    use star_invoice::{InvoiceContract, InvoiceContractClient, InvoiceStatus, ContractError};

    fn setup(env: &Env) -> (Address, Address, Address) {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();

        let freelancer = Address::generate(env);
        let client = Address::generate(env);

        (freelancer, client, token_address)
    }

    /// Test: Successful creation when amount is exactly equal to MAX_INVOICE_AMOUNT
    #[test]
    fn test_create_invoice_at_max_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Max Amount Invoice");
        let description = String::from_str(&env, "Invoice at the maximum allowed amount");
        let max_amount: i128 = 10_000_000_000_000; // MAX_INVOICE_AMOUNT

        let result = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &max_amount,
            &token_address,
            &9999999999,
            &title,
            &description,
        );

        // Should succeed
        assert!(result.is_ok(), "Invoice creation should succeed at MAX_INVOICE_AMOUNT");
        let invoice_id = result.unwrap();
        assert_eq!(invoice_id, 0, "First invoice should have ID 0");

        // Verify the invoice was created correctly
        let invoice = contract_client.get_invoice(&invoice_id).unwrap();
        assert_eq!(invoice.amount, max_amount, "Invoice amount should equal MAX_INVOICE_AMOUNT");
        assert_eq!(invoice.freelancer, freelancer);
        assert_eq!(invoice.client, client);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }

    /// Test: Failure when amount exceeds MAX_INVOICE_AMOUNT by 1
    #[test]
    fn test_create_invoice_exceeds_max_amount_by_one() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Over Max Invoice");
        let description = String::from_str(&env, "Invoice exceeding maximum amount");
        let over_max_amount: i128 = 10_000_000_000_001; // MAX_INVOICE_AMOUNT + 1

        let result = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &over_max_amount,
            &token_address,
            &9999999999,
            &title,
            &description,
        );

        // Should fail with AmountExceedsMaximum error
        assert!(result.is_err(), "Invoice creation should fail when amount exceeds MAX_INVOICE_AMOUNT");
        assert_eq!(
            result.unwrap_err().unwrap(),
            ContractError::AmountExceedsMaximum,
            "Error should be AmountExceedsMaximum"
        );
    }

    /// Test: Failure when amount exceeds MAX_INVOICE_AMOUNT by a large margin
    #[test]
    fn test_create_invoice_exceeds_max_amount_significantly() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Way Over Max Invoice");
        let description = String::from_str(&env, "Invoice far exceeding maximum amount");
        let way_over_max: i128 = 100_000_000_000_000; // 10x MAX_INVOICE_AMOUNT

        let result = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &way_over_max,
            &token_address,
            &9999999999,
            &title,
            &description,
        );

        // Should fail with AmountExceedsMaximum error
        assert!(result.is_err(), "Invoice creation should fail for significantly over-limit amounts");
        assert_eq!(
            result.unwrap_err().unwrap(),
            ContractError::AmountExceedsMaximum,
            "Error should be AmountExceedsMaximum"
        );
    }

    /// Test: Successful creation with normal amount well within the limit
    #[test]
    fn test_create_invoice_normal_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Normal Invoice");
        let description = String::from_str(&env, "Invoice with normal amount");
        let normal_amount: i128 = 1000; // Well within limit

        let result = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &normal_amount,
            &token_address,
            &9999999999,
            &title,
            &description,
        );

        // Should succeed
        assert!(result.is_ok(), "Invoice creation should succeed for normal amounts");
        let invoice_id = result.unwrap();

        // Verify the invoice was created correctly
        let invoice = contract_client.get_invoice(&invoice_id).unwrap();
        assert_eq!(invoice.amount, normal_amount);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }

    /// Test: Successful creation with moderately large amount still within limit
    #[test]
    fn test_create_invoice_large_but_valid_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Large Invoice");
        let description = String::from_str(&env, "Invoice with large but valid amount");
        let large_amount: i128 = 5_000_000_000_000; // Half of MAX_INVOICE_AMOUNT

        let result = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &large_amount,
            &token_address,
            &9999999999,
            &title,
            &description,
        );

        // Should succeed
        assert!(result.is_ok(), "Invoice creation should succeed for large but valid amounts");
        let invoice_id = result.unwrap();

        // Verify the invoice was created correctly
        let invoice = contract_client.get_invoice(&invoice_id).unwrap();
        assert_eq!(invoice.amount, large_amount);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }

    /// Test: Validation occurs before state changes (invoice ID should not increment on failure)
    #[test]
    fn test_max_amount_validation_before_state_change() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);

        // First, create a valid invoice to establish state
        let title1 = String::from_str(&env, "First Invoice");
        let description1 = String::from_str(&env, "First");
        let result1 = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &1000,
            &token_address,
            &9999999999,
            &title1,
            &description1,
        );
        assert!(result1.is_ok(), "First invoice should be created");
        let id1 = result1.unwrap();
        assert_eq!(id1, 0, "First invoice should have ID 0");

        // Now try to create an invoice with an amount that exceeds the limit
        let title2 = String::from_str(&env, "Over Limit Invoice");
        let description2 = String::from_str(&env, "This should fail");
        let result2 = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &100_000_000_000_000, // Way over limit
            &token_address,
            &9999999999,
            &title2,
            &description2,
        );
        assert!(result2.is_err(), "Second invoice creation should fail");

        // Now create another valid invoice
        // If validation occurred before state change, this should get ID 1 (not ID 2)
        let title3 = String::from_str(&env, "Third Invoice");
        let description3 = String::from_str(&env, "Third");
        let result3 = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &2000,
            &token_address,
            &9999999999,
            &title3,
            &description3,
        );
        assert!(result3.is_ok(), "Third invoice should be created");
        let id3 = result3.unwrap();
        assert_eq!(
            id3, 1,
            "Third invoice should have ID 1 (not 2), proving that the failed attempt did not increment state"
        );
    }

    /// Test: Amount validation works correctly with the minimum positive amount
    #[test]
    fn test_create_invoice_minimum_valid_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let (freelancer, client, token_address) = setup(&env);
        let title = String::from_str(&env, "Minimum Invoice");
        let description = String::from_str(&env, "Minimum valid amount");
        let min_amount: i128 = 1; // Minimum positive value

        let result = contract_client.try_create_invoice(
            &freelancer,
            &client,
            &min_amount,
            &token_address,
            &9999999999,
            &title,
            &description,
        );

        // Should succeed
        assert!(result.is_ok(), "Invoice creation should succeed with minimum valid amount");
        let invoice_id = result.unwrap();

        // Verify the invoice was created correctly
        let invoice = contract_client.get_invoice(&invoice_id).unwrap();
        assert_eq!(invoice.amount, min_amount);
        assert_eq!(invoice.status, InvoiceStatus::Pending);
    }
}
