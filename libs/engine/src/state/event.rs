use alloy::rpc::types::Block;

#[derive(Debug)]
pub enum Event<T> {
    Skip,
    Element(Box<T>),
    Checkpoint(Box<Block>),
    Many(Vec<T>),
}

#[derive(Debug)]
pub struct Events<T>(pub Vec<Event<T>>);

fn flush<T>(outcomes: &mut Vec<Event<T>>, buffer: &mut Vec<T>) {
    if !buffer.is_empty() {
        outcomes.push(Event::Many(std::mem::take(buffer)));
    }
}

pub fn batch_events<T>(events: Events<T>) -> Events<T> {
    let Events(events_vec) = events;

    let mut outcomes: Vec<Event<T>> = vec![];
    let mut buffer: Vec<T> = vec![];

    for event in events_vec {
        match event {
            Event::Skip => {
                // just ignore
            }
            Event::Element(e) => {
                buffer.push(*e);
            }
            many_events @ Event::Many(_) => {
                flush(&mut outcomes, &mut buffer);
                outcomes.push(many_events);
            }
            Event::Checkpoint(block) => {
                flush(&mut outcomes, &mut buffer);
                outcomes.push(Event::Checkpoint(block));
            }
        }
    }

    flush(&mut outcomes, &mut buffer);

    Events(outcomes)
}
