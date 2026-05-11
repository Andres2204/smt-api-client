use defmt::{info, error};
use embassy_executor::task;
use embassy_sync::pubsub::DynSubscriber;
use embassy_time::{Timer, Duration};
use crate::events::Command;

#[task]
pub async fn catch_commands(mut command_channel: DynSubscriber<'static, Command>) {
    loop {
        let command = command_channel.try_next_message_pure();
        if let Some(c) = command {
            info!("Received command {}", c);
        } else {
            error!("no commands");
        }

        Timer::after(Duration::from_secs(2)).await;
    }
}