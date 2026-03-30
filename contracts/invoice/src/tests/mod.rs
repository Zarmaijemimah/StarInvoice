pub mod helpers;

#[cfg(test)]
mod invoice_tests {
    use soroban_sdk::{testutils::Address as _, token, Address, String};

    use crate::{storage, ContractError, InvoiceContract, InvoiceContractClient};

    use super::helpers::{create_test_invoice, setup_env, TEST_AMOUNT, TEST_DEADLINE};

    // ── happy paths ──────────────────────────────────────────────────────────

    #[test]
    fn test_create_invoice() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let invoice_id = client.create_invoice(
            &freelancer,
            &payer,
            &TEST_AMOUNT,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "Logo Design"),
            &String::from_str(&env, "Logo design work"),
        );
        assert_eq!(invoice_id, 0);
    }

    #[test]
    fn test_create_invoice_unique_ids() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let title = String::from_str(&env, "Unique ID Test");
        let description = String::from_str(&env, "Unique ID test");
        for i in 0..5u64 {
            let invoice_id = client.create_invoice(
                &freelancer,
                &payer,
                &TEST_AMOUNT,
                &Address::generate(&env),
                &TEST_DEADLINE,
                &title,
                &description,
            );
            assert_eq!(invoice_id, i);
        }
        assert_eq!(client.invoice_count(), 5);
    }

    #[test]
    fn test_mark_delivered_happy_path() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        client.mark_delivered(&invoice_id);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Delivered);
    }

    #[test]
    fn test_fund_invoice_happy_path() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        client.mark_delivered(&invoice_id);
        client.approve_payment(&invoice_id);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Approved);
    }

    #[test]
    fn test_release_payment_happy_path() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, token_address) =
            create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        client.mark_delivered(&invoice_id);
        client.approve_payment(&invoice_id);
        client.release_payment(&invoice_id);
        let token_client = token::Client::new(&env, &token_address);
        assert_eq!(token_client.balance(&freelancer), TEST_AMOUNT);
        assert_eq!(token_client.balance(&contract_id), 0);
        let invoice = env.as_contract(&contract_id, || storage::get_invoice(&env, invoice_id).unwrap());
        assert_eq!(invoice.status, storage::InvoiceStatus::Completed);
    }

    // ── error codes ──────────────────────────────────────────────────────────

    #[test]
    fn test_invoice_not_found_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        assert!(client.try_get_invoice(&999).is_err());
    }

    #[test]
    fn test_invalid_amount_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let result = client.try_create_invoice(
            &freelancer,
            &payer,
            &0,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "T"),
            &String::from_str(&env, "D"),
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
    }

    #[test]
    fn test_invalid_parties_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let token = Address::generate(&env);
        let result = client.try_create_invoice(
            &freelancer,
            &freelancer,
            &TEST_AMOUNT,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "T"),
            &String::from_str(&env, "D"),
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidParties)));
    }

    #[test]
    fn test_description_too_long_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        // 257 'a' characters — one over the 256-byte limit
        let long_desc = String::from_str(
            &env,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let result = client.try_create_invoice(
            &freelancer,
            &payer,
            &TEST_AMOUNT,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "T"),
            &long_desc,
        );
        assert_eq!(result, Err(Ok(ContractError::DescriptionTooLong)));
    }

    #[test]
    fn test_invalid_status_mark_delivered_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let invoice_id = client.create_invoice(
            &freelancer,
            &payer,
            &TEST_AMOUNT,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "T"),
            &String::from_str(&env, "D"),
        );
        assert_eq!(
            client.try_mark_delivered(&invoice_id),
            Err(Ok(ContractError::InvalidInvoiceStatus))
        );
    }

    #[test]
    fn test_invalid_status_approve_payment_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        assert_eq!(
            client.try_approve_payment(&invoice_id),
            Err(Ok(ContractError::InvalidInvoiceStatus))
        );
    }

    #[test]
    fn test_invalid_status_release_payment_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        client.mark_delivered(&invoice_id);
        assert_eq!(
            client.try_release_payment(&invoice_id),
            Err(Ok(ContractError::InvalidInvoiceStatus))
        );
    }

    #[test]
    fn test_unauthorized_cancel_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let stranger = Address::generate(&env);
        let token = Address::generate(&env);
        let invoice_id = client.create_invoice(
            &freelancer,
            &payer,
            &TEST_AMOUNT,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "T"),
            &String::from_str(&env, "D"),
        );
        assert_eq!(
            client.try_cancel_invoice(&invoice_id, &stranger),
            Err(Ok(ContractError::UnauthorizedCaller))
        );
    }

    #[test]
    fn test_token_mismatch_error_code() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let token = Address::generate(&env);
        let wrong_token = Address::generate(&env);
        let invoice_id = client.create_invoice(
            &freelancer,
            &payer,
            &TEST_AMOUNT,
            &token,
            &TEST_DEADLINE,
            &String::from_str(&env, "T"),
            &String::from_str(&env, "D"),
        );
        assert_eq!(
            client.try_fund_invoice(&invoice_id, &wrong_token),
            Err(Ok(ContractError::TokenMismatch))
        );
    }

    // ── auth guards ──────────────────────────────────────────────────────────

    #[test]
    #[should_panic]
    fn test_mark_delivered_wrong_caller() {
        let env = soroban_sdk::Env::default();
        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        env.mock_all_auths();
        let (invoice_id, _) = create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        env.set_auths(&[]);
        client.mark_delivered(&invoice_id);
    }

    #[test]
    #[should_panic]
    fn test_approve_payment_wrong_caller() {
        let (env, contract_id) = setup_env();
        let client = InvoiceContractClient::new(&env, &contract_id);
        let freelancer = Address::generate(&env);
        let payer = Address::generate(&env);
        let (invoice_id, _) = create_test_invoice(&env, &client, &freelancer, &payer, TEST_AMOUNT);
        client.mark_delivered(&invoice_id);
        env.set_auths(&[]);
        client.approve_payment(&invoice_id);
    }
}
