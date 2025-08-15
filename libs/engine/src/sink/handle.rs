use eyre::Result;

#[async_trait::async_trait]
pub trait Sink: Send + Sync {
    type Item;

    async fn process(&self, element: &Self::Item) -> Result<()>;

    async fn process_batch(&self, elements: &[Self::Item]) -> Result<()>;
}
