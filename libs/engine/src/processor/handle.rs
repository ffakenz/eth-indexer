use eyre::Result;

#[async_trait::async_trait]
pub trait Processor<I, O>: Send + Sync {
    async fn process_log(&self, input: &I) -> Result<O>;
}
