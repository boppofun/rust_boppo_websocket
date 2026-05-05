use std::env;

use boppo_core::color;
use boppo_websocket::{
    ButtonEvents,
    audio::{play, stop_all_sounds},
    connect_and_setup_globals,
};

#[tokio::main]
async fn main() {
    if env::args().len() != 3 {
        panic!("usage: simple <hostname> <password>");
    }
    let url = env::args().nth(1).unwrap();
    let password = env::args().nth(2).unwrap();
    connect_and_setup_globals(&url, &password)
        .await
        .expect("failed to connect");

    let mut button_events = ButtonEvents::subscribe();
    loop {
        let event = button_events.next().await;
        if event.is_pressed() {
            event.button().set_color(color::BLUE);
            stop_all_sounds().await.ok();
            play("/effects/snare.qoa").await.ok();
        } else {
            event.button().set_color(color::OFF);
        }
    }
}
