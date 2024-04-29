use async_trait::async_trait;
use metrics::{counter, histogram};
use tracing::log::info;

use crate::api::{CaptureError, ProcessedEvent};
use crate::sinks::{DataType, Event};

pub struct PrintSink {}

#[async_trait]
impl Event for PrintSink {
    async fn send(&self, data_type: DataType, event: ProcessedEvent) -> Result<(), CaptureError> {
        info!("single {:?} event: {:?}", data_type, event);
        counter!("capture_events_ingested_total").increment(1);

        Ok(())
    }
    async fn send_batch(
        &self,
        data_type: DataType,
        events: Vec<ProcessedEvent>,
    ) -> Result<(), CaptureError> {
        let span = tracing::span!(tracing::Level::INFO, "batch of events");
        let _enter = span.enter();

        histogram!("capture_event_batch_size").record(events.len() as f64);
        counter!("capture_events_ingested_total").increment(events.len() as u64);
        for event in events {
            info!("{:?} event: {:?}", data_type, event);
        }

        Ok(())
    }
}
