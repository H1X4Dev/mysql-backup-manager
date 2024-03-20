use std::any::Any;
use std::sync::Arc;
use async_trait::async_trait;
use tokio_cron_scheduler::JobScheduler;

#[async_trait]
pub trait Service: Send + Sync + Any {
    async fn update(&self) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait ServiceScheduler {
    async fn schedule<T: Service + Any>(service: Arc<T>, sched: &mut JobScheduler, service_name: &str) -> Result<(), Box<dyn std::error::Error>>;
}