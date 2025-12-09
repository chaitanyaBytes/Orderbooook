use crate::{aggregator::Aggregator, publisher::publisher::Publisher, transformer::Transformer};
use crossbeam_channel::Receiver;
use protocol::types::Event as EngineEvent;

pub struct MarketDataPipeline {
    transformer: Transformer,
    aggregator: Aggregator,
    publishers: Vec<Box<dyn Publisher>>,
}

impl MarketDataPipeline {
    pub fn new(publishers: Vec<Box<dyn Publisher>>) -> Self {
        Self {
            transformer: Transformer::new(),
            aggregator: Aggregator::new(),
            publishers,
        }
    }

    pub fn run(&mut self, engine_rx: Receiver<EngineEvent>) {
        while let Ok(event) = engine_rx.recv() {
            let market_data_event = self.transformer.transform(event);
            let out_events = self.aggregator.process(market_data_event);

            for out in out_events {
                for p in &self.publishers {
                    p.publish(&out);
                }
            }
        }
    }
}
