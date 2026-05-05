use std::env;

use boppo_core::color;
use boppo_websocket::{
    Button, ButtonEvent, ButtonEvents,
    audio::{SoundBuilder, play, play_with_controller, stop_all_sounds},
    commands::execute_command,
    connect_and_setup_globals,
};

#[tokio::main]
async fn main() {
    if env::args().len() != 3 {
        panic!("usage: features <hostname> <password>");
    }
    let url = env::args().nth(1).unwrap();
    let password = env::args().nth(2).unwrap();
    println!("Connecting to {url:?} ...");
    connect_and_setup_globals(&url, &password)
        .await
        .expect("failed to connect");
    println!("Connected!");

    const FLASH_BUTTON: Button = Button::B0;
    const PRESS_LIGHT_BUTTON: Button = Button::B1;
    const SFX_SIMPLE_BUTTON: Button = Button::B5;
    const SFX_WAIT_BUTTON: Button = Button::B6;
    const SFX_ERROR_BUTTON: Button = Button::B7;
    const STOP_ALL_SOUNDS_BUTTON: Button = Button::B8;
    const SLEEP_BUTTON: Button = Button::B9;

    const PAUSE_BUTTON: Button = Button::B2;
    const SPEED_CHANGE_BUTTON: Button = Button::B3;
    const VOLUME_CHANGE_BUTTON: Button = Button::B4;

    SFX_SIMPLE_BUTTON.set_color(color::ORANGE);
    SFX_WAIT_BUTTON.set_color(color::ORANGE);
    SFX_ERROR_BUTTON.set_color(color::RED);
    PAUSE_BUTTON.set_color(color::GREEN);
    SPEED_CHANGE_BUTTON.set_color(color::PURPLE);
    VOLUME_CHANGE_BUTTON.set_color(color::YELLOW);
    STOP_ALL_SOUNDS_BUTTON.set_color(color::WHITE);
    SLEEP_BUTTON.set_color(color::GREY);

    tokio::spawn(async move {
        let mut button_events = ButtonEvents::subscribe();
        let music_controller =
            play_with_controller(SoundBuilder::file("music/Being_Me.mp3").repeat_forever())
                .await
                .unwrap();

        music_controller.set_paused(true);

        let mut paused = true;
        let mut volume = 1.0;
        let mut speed = 1.0;

        loop {
            let event = button_events.next().await;
            print_event(event);
            let button = event.button();
            if event.is_released() && button != PRESS_LIGHT_BUTTON {
                continue;
            }
            match button {
                PRESS_LIGHT_BUTTON => {
                    let color = if event.is_pressed() {
                        color::RED
                    } else {
                        color::OFF
                    };
                    PRESS_LIGHT_BUTTON.set_color(color);
                }
                SFX_SIMPLE_BUTTON => {
                    play("/effects/success.qoa").await.ok();
                }
                SFX_ERROR_BUTTON => {
                    play("/effects/MISSING_SOUND_FOR_ERROR.qoa").await.ok();
                }
                SFX_WAIT_BUTTON => {
                    let controller = play_with_controller("music/add_music_instructions.mp3")
                        .await
                        .unwrap();
                    SFX_WAIT_BUTTON.set_color(color::BLUE);
                    tokio::spawn(async move {
                        controller.wait_until_finished().await;
                        SFX_WAIT_BUTTON.set_color(color::ORANGE);
                    });
                }
                PAUSE_BUTTON => {
                    paused = !paused;
                    let color = if paused { color::GREEN } else { color::RED };
                    PAUSE_BUTTON.set_color(color);
                    music_controller.set_paused(paused);
                }
                VOLUME_CHANGE_BUTTON => {
                    volume = match volume {
                        1.0 => 0.5,
                        _ => 1.0,
                    };
                    music_controller.set_volume(volume);
                }
                SPEED_CHANGE_BUTTON => {
                    speed = match speed {
                        1.0 => 2.0,
                        2.0 => 0.5,
                        _ => 1.0,
                    };
                    music_controller.set_speed(speed);
                }
                STOP_ALL_SOUNDS_BUTTON => {
                    stop_all_sounds().await.ok();
                }
                SLEEP_BUTTON => {
                    execute_command("sleep").await.ok();
                }
                _ => {}
            }
        }
    });

    loop {
        FLASH_BUTTON.set_color(color::RED);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        FLASH_BUTTON.set_color(color::OFF);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

fn print_event(event: ButtonEvent) {
    let action = if event.is_pressed() {
        "pressed"
    } else {
        "released"
    };
    println!("tablet event: button {} {action}", event.button().index());
}
