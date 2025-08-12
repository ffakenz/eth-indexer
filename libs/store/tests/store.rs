#[cfg(test)]
mod tests {
    use store::store::add;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
