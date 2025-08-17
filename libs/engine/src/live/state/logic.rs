use crate::live::{
    source::handle::SourceInput,
    state::{event::Event, outcome::Outcome},
};
use alloy::rpc::types::Block;
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

    pub fn increment_event_counter(&mut self) {
        self.event_counter += 1;
    }

    pub fn reset_event_counter(&mut self) {
        self.event_counter = 0;
    }

    pub fn set_next_checkpoint_block_number(&mut self, block_number: u64) {
        self.next_checkpoint_block_number = block_number;
    }

    pub fn get_next_checkpoint_block_number(&self) -> u64 {
        self.next_checkpoint_block_number
    }

    // Every N logs, produce a checkpoint event (skip first iteration)
    pub fn checkpoint_decision(&self, interval: u64) -> (bool, u64) {
        (
            self.event_counter > 0 && self.event_counter == interval,
            self.next_checkpoint_block_number,
        )
    }

    pub fn on_checkpoint<T>(&mut self, maybe_checkpoint_block: Option<Block>) -> Result<Event<T>> {
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

    pub fn on_input<E, T>(&mut self, input: E) -> Result<Event<T>>
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
