//! Shell command execution on the tablet.

use crate::globals::SENDER;

/// Execute a shell command on the tablet.
///
/// See the full list of supported commands at [developer.boppo.com/docs/commands](https://developer.boppo.com/docs/commands)
///
/// # Panics
///
/// Panics if [`crate::setup_globals`] has not been called.
pub async fn execute_command(command: &str) -> Result<(), crate::Error> {
    let mut sender_guard = SENDER.lock().await;
    let sender = sender_guard.as_mut().unwrap();
    sender.execute_command(command).await
}
