use crate::live::{
    source::handle::SourceInput,
    state::{
        event::{Event, Events},
        outcome::Outcome,
    },
};
use chain::rpc::NodeClient;
use eyre::Result;
use std::fmt::Debug;

#[derive(Debug)]
pub struct State {
    block_counter: u64,
    current_block_number: u64,
}

impl State {
    pub fn new(current_block_number: u64) -> Self {
        Self { block_counter: 0, current_block_number }
    }

    pub fn get_next_checkpoint_block_number(&self) -> u64 {
        self.current_block_number + 1
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
        match input.clone().try_into() {
            Err(e) => {
                tracing::error!("Skip: Failed to convert consumed input: {input:?} - reason {e:?}");
                Ok(Events(vec![Event::Skip]))
            }
            Ok(t) => {
                tracing::info!("Publisher rolling forward: {input:?}");
                if t.block_number() == self.current_block_number {
                    let outcome_event = Event::Element(Box::new(t));
                    Ok(Events(vec![outcome_event]))
                }
                // TODO! handle fork (reorg)
                else {
                    self.set_current_block_number(t.block_number());
                    self.increment_block_counter();

                    let outcome_event = Event::Element(Box::new(t));
                    let do_checkpoint =
                        // Every N blocks, produce a checkpoint event (skip first iteration)
                        self.block_counter > 0 && self.block_counter == checkpoint_interval;
                    if !do_checkpoint {
                        Ok(Events(vec![outcome_event]))
                    } else {
                        let checkpoint_block_number = self.current_block_number;
                        match node_client.get_block_by_number(self.current_block_number).await? {
                            None => Ok(Events(vec![outcome_event])),
                            Some(checkpoint_block) => {
                                tracing::info!(
                                    "Publisher checkpointing: {checkpoint_block_number:?}"
                                );
                                self.reset_block_counter();
                                let checkpoint_event =
                                    Event::Checkpoint(Box::new(checkpoint_block));
                                Ok(Events(vec![checkpoint_event, outcome_event]))
                            }
                        }
                    }
                }
            }
        }
    }

    fn increment_block_counter(&mut self) {
        self.block_counter += 1;
    }

    fn reset_block_counter(&mut self) {
        self.block_counter = 0;
    }

    fn set_current_block_number(&mut self, block_number: u64) {
        self.current_block_number = block_number;
    }
}
