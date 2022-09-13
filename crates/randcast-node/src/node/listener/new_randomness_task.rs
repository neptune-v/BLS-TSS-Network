use super::Listener;
use crate::node::{
    contract_client::rpc_mock::adapter::{AdapterMockHelper, MockAdapterClient},
    dal::{types::ChainIdentity, BLSTasksFetcher},
    dal::{types::RandomnessTask, BLSTasksUpdater},
    error::{NodeError, NodeResult},
    event::new_randomness_task::NewRandomnessTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct MockNewRandomnessTaskListener<
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
> {
    chain_id: usize,
    id_address: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>>
    MockNewRandomnessTaskListener<T>
{
    pub fn new(
        chain_id: usize,
        id_address: String,
        chain_identity: Arc<RwLock<ChainIdentity>>,
        randomness_tasks_cache: Arc<RwLock<T>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        MockNewRandomnessTaskListener {
            chain_id,
            id_address,
            chain_identity,
            randomness_tasks_cache,
            eq,
        }
    }
}

impl<T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>>
    EventPublisher<NewRandomnessTask> for MockNewRandomnessTaskListener<T>
{
    fn publish(&self, event: NewRandomnessTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send> Listener
    for MockNewRandomnessTaskListener<T>
{
    async fn start(mut self) -> NodeResult<()> {
        let rpc_endpoint = self
            .chain_identity
            .read()
            .get_provider_rpc_endpoint()
            .to_string();

        let client = MockAdapterClient::new(rpc_endpoint, self.id_address.to_string());

        let retry_strategy = FixedInterval::from_millis(2000);

        loop {
            if let Err(err) = RetryIf::spawn(
                retry_strategy.clone(),
                || async {
                    let task_reply = client.emit_signature_task().await;

                    if let Ok(randomness_task) = task_reply {
                        if let Ok(false) = self
                            .randomness_tasks_cache
                            .read()
                            .contains(randomness_task.index)
                        {
                            info!("received new randomness task. {:?}", randomness_task);

                            self.randomness_tasks_cache
                                .write()
                                .add(randomness_task.clone())?;

                            self.publish(NewRandomnessTask::new(self.chain_id, randomness_task));
                        }
                    }

                    NodeResult::Ok(())
                },
                |e: &NodeError| {
                    error!("listener is interrupted. Retry... Error: {:?}, ", e);
                    true
                },
            )
            .await
            {
                error!("{:?}", err);
            }

            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
    }
}