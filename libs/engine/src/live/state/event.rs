use alloy::rpc::types::Block;

#[derive(Debug)]
pub enum Event<T> {
    Skip,
    Element(Box<T>),
    Checkpoint(Box<Block>),
}

#[derive(Debug)]
pub struct Events<T>(pub Vec<Event<T>>);
