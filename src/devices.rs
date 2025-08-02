use anyhow::{Context, Result, anyhow};
use devicectrl_common::UpdateRequest;
use evdev::{Device, KeyCode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::CreateKind};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

use crate::config::Config;

pub async fn listen_events(
    device: Device,
    config: Arc<Config>,
    update_sender: mpsc::Sender<UpdateRequest>,
) -> Result<()> {
    let name = device.name().unwrap_or("unknown").to_owned();

    let mut events = device
        .into_event_stream()
        .context("Failed to get event stream")?;

    log::info!("Listening for events on [{name}]");

    loop {
        let event = events.next_event().await?;

        if event.value() != 1 {
            continue;
        }

        let code = KeyCode::new(event.code());

        let actions = config
            .actions
            .iter()
            .filter(|(trigger, _)| {
                code == trigger.key
                    && trigger.value.is_none_or(|v| v == event.value())
                    && trigger
                        .device_names
                        .as_ref()
                        .is_none_or(|names| names.contains(&name))
            })
            .flat_map(|a| a.1.clone())
            .collect::<Vec<_>>();

        if actions.is_empty() {
            log::warn!("Unhandled input on [{name}]: {code:?}");
            continue;
        };

        for action in actions {
            if let Err(err) = update_sender.send(action).await {
                log::error!("{:?}", anyhow!(err).context("Failed to send input action"));
            }
        }
    }
}

pub async fn monitor_devices(
    config: Arc<Config>,
    update_sender: mpsc::Sender<UpdateRequest>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(5);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(err) = tx.blocking_send(res) {
                log::error!(
                    "{:?}",
                    anyhow!(err).context("Failed to add input to channel")
                );
            }
        },
        notify::Config::default(),
    )?;

    watcher
        .watch(Path::new("/dev/input/"), RecursiveMode::NonRecursive)
        .context("Failed to watch directory")?;

    while let Some(event) = rx.recv().await {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                log::error!("Failed to handle filesystem event: {err:?}");
                continue;
            }
        };

        if !matches!(event.kind, EventKind::Create(CreateKind::File)) {
            continue;
        }

        event
            .paths
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .is_some_and(|p| p.to_string_lossy().starts_with("event"))
            })
            .for_each(|path| match Device::open(&path) {
                Ok(device) => {
                    let config = config.clone();
                    let update_sender = update_sender.clone();
                    tokio::spawn(async move {
                        if let Err(err) = listen_events(device, config, update_sender).await {
                            log::error!(
                                "{:?}",
                                err.context(format!("Failed to listen to events on [{path:?}]"))
                            );
                        }
                    });
                }
                Err(err) => {
                    log::error!("{:?}", anyhow!(err).context("Failed to open device"));
                }
            })
    }
    Ok(())
}
