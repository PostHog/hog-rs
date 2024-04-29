use async_trait::async_trait;

use crate::api::{CaptureError, ProcessedEvent};

pub mod kafka;
pub mod print;

#[derive(Debug, Copy, Clone)]
pub enum DataType {
    AnalyticsMain,
    AnalyticsOverflow,
    AnalyticsHistorical,
}

#[async_trait]
pub trait Event {
    async fn send(&self, data_type: DataType, event: ProcessedEvent) -> Result<(), CaptureError>;
    async fn send_batch(
        &self,
        data_type: DataType,
        events: Vec<ProcessedEvent>,
    ) -> Result<(), CaptureError>;
}
