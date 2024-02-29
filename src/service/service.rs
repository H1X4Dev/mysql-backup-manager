use tokio_cron_scheduler::JobScheduler;

pub trait Service {
    async fn update(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn schedule(&self, sched: &mut JobScheduler, service_name: &str) -> Result<(), Box<dyn std::error::Error>>;
}