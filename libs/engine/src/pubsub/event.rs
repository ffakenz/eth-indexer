use alloy::rpc::types::Block;

#[derive(Debug)]
pub enum Event<E> {
    Skip,
    Input(Box<E>),
    Checkpoint(Box<Block>),
}
