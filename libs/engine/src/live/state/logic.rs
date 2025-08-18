use crate::{
    checkpointer::Checkpointer,
    live::{
        source::handle::SourceInput,
        state::{
            event::{self, Event, Events},
            outcome::Outcome,
        },
    },
};
use alloy::{primitives::BlockNumber, rpc::types::Block};
use chain::rpc::NodeClient;
use eyre::Result;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct State {
    block_counter: u64,
    checkpoint_counter: u64,
    current_block_number: u64,
}

pub async fn init_state(
    block_tip: &Block,
    from_block: Option<u64>,
    checkpointer: &Checkpointer,
) -> Result<State> {
    let checkpoint_number: BlockNumber = match from_block {
        // use from_block if given
        Some(from_block_number) => from_block_number,
        None => match checkpointer.get_last_checkpoint().await? {
            // use latest known if not
            Some(checkpoint_block) => checkpoint_block.block_number as u64,
            // if latest known does not exist, start from the tip
            None => block_tip.number(),
        },
    };

    Ok(State::new(checkpoint_number))
}

impl State {
    pub fn new(current_block_number: u64) -> Self {
        Self { block_counter: 0, checkpoint_counter: 0, current_block_number }
    }

    pub fn get_checkpoint_counter(&self) -> u64 {
        self.checkpoint_counter
    }

    pub fn get_current_block_number(&self) -> u64 {
        self.current_block_number
    }

    pub async fn flush_checkpoint<T>(&mut self, node_client: &NodeClient) -> Result<Event<T>> {
        let in_memory_block_number = self.current_block_number;
        match node_client.get_block_by_number(in_memory_block_number).await? {
            None => Ok(Event::Skip),
            Some(checkpoint_block) => {
                tracing::info!("Logic Checkpointing: {in_memory_block_number:?}");
                self.increment_checkpoint_counter();
                self.reset_block_counter();
                let checkpoint_event = Event::Checkpoint(Box::new(checkpoint_block));
                Ok(checkpoint_event)
            }
        }
    }

    pub async fn roll_forward_batch<E, T>(
        &mut self,
        inputs: Vec<E>,
        checkpoint_interval: u64,
        node_client: &NodeClient,
    ) -> Result<Events<T>>
    where
        E: SourceInput + TryInto<T> + Clone + Debug,
        <E as TryInto<T>>::Error: Debug,
        T: Outcome + TryFrom<E>,
    {
        let mut outcomes: Vec<Event<T>> = vec![];
        for input in inputs {
            let Events(events_vec) =
                self.roll_forward(input, checkpoint_interval, node_client).await?;
            outcomes.extend(events_vec);
        }
        Ok(event::batch_events(Events(outcomes)))
    }

    pub async fn roll_forward<E, T>(
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
        match input.clone().try_into() {
            Err(e) => {
                tracing::error!("Skip: Failed to convert sourced input: {input:?} - reason {e:?}");
                Ok(Events(vec![Event::Skip]))
            }
            Ok(t) => {
                tracing::info!("Logic rolling forward: {input:?}");
                if t.block_number() == self.current_block_number {
                    let outcome_event = Event::Element(Box::new(t));
                    Ok(Events(vec![outcome_event]))
                }
                // TODO! handle fork (reorg)
                else {
                    self.set_current_block_number(t.block_number());
                    self.increment_block_counter();

                    let outcome_event = Event::Element(Box::new(t));
                    // Every N blocks, produce a checkpoint event (skip first iteration)
                    let do_checkpoint =
                        self.block_counter > 0 && self.block_counter == checkpoint_interval;
                    if !do_checkpoint {
                        Ok(Events(vec![outcome_event]))
                    } else {
                        let checkpoint_event = self.flush_checkpoint(node_client).await?;
                        Ok(Events(vec![checkpoint_event, outcome_event]))
                    }
                }
            }
        }
    }

    fn increment_block_counter(&mut self) {
        self.block_counter += 1;
    }

    fn increment_checkpoint_counter(&mut self) {
        self.checkpoint_counter += 1;
    }

    fn reset_block_counter(&mut self) {
        self.block_counter = 0;
    }

    fn set_current_block_number(&mut self, block_number: u64) {
        self.current_block_number = block_number;
    }
}
