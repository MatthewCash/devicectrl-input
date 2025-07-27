use anyhow::{Context, Result};
use devicectrl_common::UpdateRequest;
use std::{env, path::PathBuf, sync::Arc};

use sd_notify::NotifyState;
use tokio::{sync::mpsc, task::JoinSet};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use crate::{
    devices::{listen_events, monitor_devices},
    transport::start_communication,
};

mod config;
mod devices;
mod transport;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .without_time() // systemd logs already include timestamps
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .with_env_var("LOG_LEVEL")
                .from_env()?,
        )
        .init();

    let config = Arc::new(
        config::load_config(&PathBuf::from(
            env::var("CONFIG_PATH").expect("CONFIG_PATH env var missing!"),
        ))
        .await
        .context("failed to load config")?,
    );

    let (sender, receiver) = mpsc::channel::<UpdateRequest>(64);

    let mut tasks = JoinSet::new();

    tasks.spawn(start_communication(
        config.server_connection.clone(),
        receiver,
    ));

    evdev::enumerate().for_each(|(_, device)| {
        tasks.spawn(listen_events(device, config.clone(), sender.clone()));
    });

    tasks.spawn(monitor_devices(config, sender.clone()));

    let _ = sd_notify::notify(false, &[NotifyState::Ready]);

    while tasks.join_next().await.transpose()?.transpose()?.is_some() {}

    Ok(())
}
