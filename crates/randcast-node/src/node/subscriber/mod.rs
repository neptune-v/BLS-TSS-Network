pub mod block;
pub mod group_relay_confirmation_signature_aggregation;
pub mod group_relay_signature_aggregation;
pub mod in_grouping;
pub mod post_grouping;
pub mod post_success_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_group_relay_confirmation_task;
pub mod ready_to_handle_group_relay_task;
pub mod ready_to_handle_randomness_task;

use crate::node::{
    error::NodeResult,
    event::{types::Topic, Event},
};

pub trait Subscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()>;

    fn subscribe(self);
}
