#[cfg(test)]
mod tests {
    use soroban_sdk::Env;
    use star_invoice::{InvoiceContract, InvoiceContractClient};

    #[test]
    fn test_get_nonexistent_invoice_returns_error() {
        let env = Env::default();
        let contract_id = env.register_contract(None, InvoiceContract);
        let contract_client = InvoiceContractClient::new(&env, &contract_id);

        let result = contract_client.try_get_invoice(&9999);
        assert!(result.is_err());
    }
}
