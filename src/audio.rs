//! Audio playback functions

pub use boppo_core::audio::*;

use crate::{client::SetSoundParameterRequest, globals::SENDER};

/// Play a sound on the tablet.
///
/// This will not return an error if the sound fails to play on the tablet but
/// an error will be returned and printed as an error message asynchronously.
///
/// # Panics
///
/// Panics if [`crate::setup_globals`] has not been called.
pub async fn play(sound: impl Into<SoundBuilder>) -> Result<(), crate::Error> {
    let mut sender_guard = SENDER.lock().await;
    let sender = sender_guard.as_mut().unwrap();
    sender.play_sound(sound.into().as_instruction()).await?;
    Ok(())
}

/// Wrap `sound` with a controller and play it.
///
/// This is a convenience wrapper around [`SoundBuilder::controller`] and [`play`].
///
/// # Panics
///
/// Panics if [`crate::setup_globals`] has not been called.
pub async fn play_with_controller(
    sound: impl Into<SoundBuilder>,
) -> Result<Controller, crate::Error> {
    let (sound, controller) = sound.into().controller();
    play(sound).await?;
    Ok(controller)
}

/// Play `sound` and wait until it finishes.
///
/// This is a convenience wrapper around [`SoundBuilder::controller`], [`play`], and [`Controller::wait_until_finished`].
///
/// # Panics
///
/// Panics if [`crate::setup_globals`] has not been called.
pub async fn play_and_wait_until_finished(
    sound: impl Into<SoundBuilder>,
) -> Result<(), crate::Error> {
    play_with_controller(sound)
        .await?
        .wait_until_finished()
        .await;
    Ok(())
}

/// Modify parameters of a currently-playing sound.
///
/// Normally this is done via a [`Controller`] returned by [`play_with_controller`], but
/// this function lets you send raw parameter changes when you hold the ID directly.
///
/// # Panics
///
/// Panics if [`crate::setup_globals`] has not been called.
pub async fn modify_sound(req: SetSoundParameterRequest) -> Result<(), crate::Error> {
    let mut sender_guard = SENDER.lock().await;
    let sender = sender_guard.as_mut().unwrap();
    sender.set_sound_parameter(req).await
}

/// Stop all currently-playing sounds.
///
/// # Panics
///
/// Panics if [`crate::setup_globals`] has not been called.
pub async fn stop_all_sounds() -> Result<(), crate::Error> {
    let mut sender_guard = SENDER.lock().await;
    let sender = sender_guard.as_mut().unwrap();
    sender.stop_all_sounds().await
}
