#[cfg(test)]
mod tests {
    use sync::consumer;
    use sync::producer;

    #[test]
    fn it_works() {
        let result = producer::add(2, 2);
        assert_eq!(result, 4);
        let result = consumer::add(2, 2);
        assert_eq!(result, 4);
    }
}
