use crate::live::{
    source::handle::SourceInput,
    state::{
        event::{Event, Events},
        outcome::Outcome,
    },
};
use alloy::rpc::types::Block;
use chain::rpc::NodeClient;
use eyre::Result;
use std::fmt::Debug;

#[derive(Debug)]
pub struct State {
    event_counter: u64,
    next_checkpoint_block_number: u64,
}

impl State {
    pub fn new(next_checkpoint_block_number: u64) -> Self {
        Self { event_counter: 0, next_checkpoint_block_number }
    }

    pub fn get_next_checkpoint_block_number(&self) -> u64 {
        self.next_checkpoint_block_number
    }

    pub async fn on_roll_forward<E, T>(
        &mut self,
        input: E,
        checkpoint_interval: u64,
        node_client: &NodeClient,
    ) -> Result<Events<T>>
    where
        E: SourceInput + TryInto<T> + Clone + Debug,
        <E as TryInto<T>>::Error: Debug,
        T: Outcome + TryFrom<E>,
    {
        let (do_checkpoint, checkpoint_block_number) =
            self.checkpoint_decision(checkpoint_interval);
        if do_checkpoint {
            tracing::info!("Publisher checkpointing: {checkpoint_block_number:?}");
            let maybe_checkpoint_block =
                node_client.get_block_by_number(checkpoint_block_number).await?;
            let checkpoint_event = self.on_checkpoint(maybe_checkpoint_block)?;
            let input_event = self.on_input(input)?;
            Ok(Events(vec![checkpoint_event, input_event]))
        } else {
            tracing::info!("Publisher rolling forward: {input:?}");
            let input_event = self.on_input(input)?;
            Ok(Events(vec![input_event]))
        }
    }

    fn increment_event_counter(&mut self) {
        self.event_counter += 1;
    }

    fn reset_event_counter(&mut self) {
        self.event_counter = 0;
    }

    fn set_next_checkpoint_block_number(&mut self, block_number: u64) {
        self.next_checkpoint_block_number = block_number;
    }

    // Every N logs, produce a checkpoint event (skip first iteration)
    fn checkpoint_decision(&self, interval: u64) -> (bool, u64) {
        (
            self.event_counter > 0 && self.event_counter == interval,
            self.next_checkpoint_block_number,
        )
    }

    fn on_checkpoint<T>(&mut self, maybe_checkpoint_block: Option<Block>) -> Result<Event<T>> {
        match maybe_checkpoint_block {
            None => {
                self.increment_event_counter();
                Ok(Event::Skip)
            }
            Some(checkpoint_block) => {
                self.reset_event_counter();
                Ok(Event::Checkpoint(Box::new(checkpoint_block)))
            }
        }
    }

    fn on_input<E, T>(&mut self, input: E) -> Result<Event<T>>
    where
        E: SourceInput + TryInto<T> + Clone + Debug,
        <E as TryInto<T>>::Error: Debug,
        T: Outcome + TryFrom<E>,
    {
        match input.clone().try_into() {
            Err(e) => {
                tracing::error!("Skip: Failed to convert consumed input: {input:?} - reason {e:?}");
                self.increment_event_counter();
                Ok(Event::Skip)
            }
            Ok(t) => {
                self.set_next_checkpoint_block_number(t.block_number());
                self.increment_event_counter();
                Ok(Event::Element(Box::new(t)))
            }
        }
    }
}
