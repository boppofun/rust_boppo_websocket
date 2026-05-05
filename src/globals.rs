use std::sync::{Mutex, Once, OnceLock};

use boppo_core::{
    Framebuffer, Lights,
    color::RGB,
    executor::Executor,
    internal::{AudioParameter, ButtonCounts},
    log::error,
};
use tokio::sync::{Mutex as AsyncMutex, watch};

use crate::{
    ButtonEvent,
    client::{BoppoReceiver, BoppoSender, SetSoundParameterRequest},
};

pub(crate) static SENDER: AsyncMutex<Option<BoppoSender>> = AsyncMutex::const_new(None);
static RECEIVER_TASK: Mutex<Option<tokio::task::AbortHandle>> = Mutex::new(None);
static BUTTON_TX: OnceLock<tokio::sync::broadcast::Sender<ButtonEvent>> = OnceLock::new();
static BUTTON_COUNTS_SENDER: OnceLock<watch::Sender<ButtonCounts>> = OnceLock::new();
static HAL_INIT: Once = Once::new();

/// Connect to a tablet and initialize `boppo_core` globals in one step.
///
/// Equivalent to calling [`client::connect`][crate::client::connect] followed
/// by [`setup_globals`] with an `on_disconnect` handler that calls [`std::process::exit`].
pub async fn connect_and_setup_globals(hostname: &str, password: &str) -> Result<(), crate::Error> {
    let (sender, receiver) = crate::client::connect(hostname, password).await?;
    setup_globals(sender, receiver, || std::process::exit(1)).await;
    Ok(())
}

/// Wire a tablet connection into `boppo_core` globals.
///
/// Enables high-level `boppo_core` APIs such as `Button::B0.set_color(...)`,
/// sets up the edge-executor so `boppo_core::executor` works, and initializes
/// the audio subsystem. Safe to call again after a reconnect — the previous
/// receiver task is aborted and the sender is replaced.
pub async fn setup_globals(
    sender: BoppoSender,
    mut receiver: BoppoReceiver,
    on_disconnect: impl Fn() + Send + 'static,
) {
    *SENDER.lock().await = Some(sender);

    if let Some(handle) = RECEIVER_TASK.lock().unwrap().take() {
        handle.abort();
    }

    HAL_INIT.call_once(|| {
        boppo_core::internal::set_lights(set_lights_impl);
        let (button_tx, _) = tokio::sync::broadcast::channel(64);
        boppo_core::internal::set_button_events(button_tx.clone());
        BUTTON_TX.set(button_tx).unwrap();

        let (button_counts_sender, button_counts_receiver) =
            watch::channel(ButtonCounts::default());
        BUTTON_COUNTS_SENDER.set(button_counts_sender).unwrap();
        boppo_core::internal::set_button_counts(button_counts_receiver);

        // Setup the edge-executor alongside tokio so boppo::executor works
        std::thread::Builder::new()
            .name("edge-executor runtime".to_string())
            .spawn(|| {
                let executor: &'static mut Executor = Box::leak(Box::new(Executor::new()));
                boppo_core::internal::set_executor(executor);
                edge_executor::block_on(executor.run(core::future::pending::<()>()));
            })
            .unwrap();

        boppo_core::internal::init_audio(modify_controller_param);
    });

    let button_tx = BUTTON_TX.get().unwrap().clone();
    let button_counts_tx = BUTTON_COUNTS_SENDER.get().unwrap().clone();
    let handle = tokio::spawn(async move {
        loop {
            match receiver.next_event().await {
                Ok(Some(crate::client::TabletEvent::Button(event))) => {
                    button_tx.send(event).ok();
                    button_counts_tx.send_modify(|counts| {
                        counts.update_for_event(event.button(), event.is_pressed());
                    });
                }
                Ok(Some(crate::client::TabletEvent::SoundFinished(id))) => {
                    boppo_core::internal::on_sound_controller_finished(id);
                }
                Ok(Some(crate::client::TabletEvent::ErrorMessage(msg))) => {
                    eprintln!("error from tablet: {}", msg);
                }
                Err(e) => {
                    eprintln!("error receiving message: {}", e);
                    on_disconnect();
                    return;
                }
                Ok(None) => {
                    eprintln!("connection to tablet closed");
                    on_disconnect();
                    return;
                }
            }
        }
    });
    *RECEIVER_TASK.lock().unwrap() = Some(handle.abort_handle());
}

fn set_lights_impl(colors: &[RGB; Lights::COUNT]) {
    let colors = *colors;
    tokio::spawn(async move {
        if let Some(sender) = SENDER.lock().await.as_mut() {
            sender.set_lights(Framebuffer { colors }).await.ok();
        }
    });
}

fn modify_controller_param(id: u64, param: boppo_core::internal::AudioParameter, value: f32) {
    let mut req = SetSoundParameterRequest::new(id);
    match param {
        AudioParameter::Volume => req.volume = Some(value),
        AudioParameter::Speed => req.speed = Some(value),
        AudioParameter::Pause => req.pause = Some(value != 0.0),
        AudioParameter::Stop => req.stop = true,
    }
    tokio::spawn(async move {
        if let Err(e) = crate::audio::modify_sound(req).await {
            error!("failed to set sound parameter: {e}");
        };
    });
}
