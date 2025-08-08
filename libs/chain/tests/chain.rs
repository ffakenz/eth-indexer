#[cfg(test)]
mod tests {
    use chain::rpc_provider;

    #[test]
    fn it_works() {
        let result = rpc_provider::add(2, 2);
        assert_eq!(result, 4);
    }
}
