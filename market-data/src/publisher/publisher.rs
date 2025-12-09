use crate::types::Event;

pub trait Publisher: Send + Sync {
    fn publish(&self, event: &Event);
    fn publish_batch(&self, events: Vec<Event>) {
        for e in events {
            self.publish(&e)
        }
    }
}
