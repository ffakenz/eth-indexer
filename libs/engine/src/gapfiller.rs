use crate::args::Args;
use crate::checkpointer::Checkpointer;
use crate::live::pubsub::subscriber;
use crate::live::sink::handle::Sink;
use crate::live::source::filter::ChunkFilter;
use crate::live::source::handle::{Source, SourceInput};
use crate::live::state::event::Events;
use crate::live::state::logic::State;
use crate::live::state::outcome::Outcome;
use alloy::rpc::types::Block;
use chain::rpc::NodeClient;
use eyre::{Result, eyre};
use std::fmt::Debug;

pub struct Gapfiller<'a, E, T> {
    node_client: &'a NodeClient,
    source: &'a dyn Source<Item = E>,
    checkpointer: &'a Checkpointer,
    sink: &'a dyn Sink<Item = T>,
}

impl<'a, E, T> Gapfiller<'a, E, T> {
    pub fn new(
        node_client: &'a NodeClient,
        source: &'a dyn Source<Item = E>,
        checkpointer: &'a Checkpointer,
        sink: &'a dyn Sink<Item = T>,
    ) -> Self {
        Self { node_client, source, checkpointer, sink }
    }

    pub async fn chunked_backfill(
        &self,
        args: &Args,
        block_tip: Block,
        state: &mut State,
    ) -> Result<()>
    where
        E: SourceInput + TryInto<T> + Debug + Clone,
        <E as TryInto<T>>::Error: Debug,
        T: Outcome + TryFrom<E> + Debug,
    {
        let checkpoint_interval = match args.backfill_checkpoint_interval {
            None => args.checkpoint_interval,
            Some(backfill_checkpoint_interval) => backfill_checkpoint_interval,
        };

        let latest_block_number = block_tip.number();
        let mut from_block_number = state.get_current_block_number() + 1;
        tracing::info!("Backfill started at block number: {from_block_number:?}");

        // Process historical chunks until we reach the snapshot tip
        while from_block_number <= latest_block_number {
            // A safe checked addition avoids silent wraparound
            let chunk_block_number =
                from_block_number.saturating_add(checkpoint_interval - 1).min(latest_block_number);

            let chunk_filter = ChunkFilter {
                addresses: args.addresses.clone(),
                event: args.event.clone(),
                from_block_number: from_block_number.into(),
                to_block_number: chunk_block_number.into(),
            };
            let source_inputs = self.source.chunk(chunk_filter).await?;

            let Events(batched_outcomes) = state
                .roll_forward_batch(source_inputs, checkpoint_interval, self.node_client)
                .await?;

            for outcome in batched_outcomes {
                match subscriber::consume_event_outcome(outcome, self.checkpointer, self.sink).await
                {
                    Ok(success) => success,
                    Err(e) => {
                        tracing::error!("Backfill failed: {e:?}");
                        return Err(eyre!(e));
                    }
                }
            }

            from_block_number = chunk_block_number + 1;
        }

        tracing::info!("Backfill finished at block number: {latest_block_number:?}");

        // note we flush a checkpoint at the end of backfilling
        if state.get_checkpoint_counter() == 0 {
            let checkpoint_outcome = state.flush_checkpoint(self.node_client).await?;
            subscriber::consume_event_outcome(checkpoint_outcome, self.checkpointer, self.sink)
                .await?
        }
        Ok(())
    }
}
