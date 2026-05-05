//! Direct WebSocket connection to a Boppo tablet.
//!
//! Use [`connect`] to obtain a [`BoppoSender`] / [`BoppoReceiver`] pair.
//! This module has no global state, so multiple simultaneous tablet connections
//! are supported.

use std::{num::ParseIntError, sync::Arc};

use boppo_core::{
    Button, ButtonEvent, Buttons, Framebuffer,
    audio::SoundInstruction,
    log::{debug, error},
};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
    tungstenite::{Message, client::IntoClientRequest, http::header},
};

use serde::Serialize;

use crate::Error;

/// Request to modify parameters of a currently-playing sound controller.
///
/// Build with [`SetSoundParameterRequest::new`] and the builder methods, then
/// pass to [`crate::audio::modify_sound`] or [`BoppoSender::set_sound_parameter`].
#[derive(Debug, Clone, Serialize)]
pub struct SetSoundParameterRequest {
    /// ID of the sound controller to modify.
    pub controller_id: u64,
    /// `Some(true)` to pause, `Some(false)` to unpause, `None` to leave unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pause: Option<bool>,
    /// `true` to stop and destroy the controller.
    #[serde(skip_serializing_if = "is_false")]
    pub stop: bool,
    /// Volume level override, typically `0.0`–`1.0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
    /// Playback speed multiplier; `1.0` is normal speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

impl SetSoundParameterRequest {
    /// Create a request for `controller_id` with no modifications set.
    pub fn new(controller_id: u64) -> Self {
        Self {
            controller_id,
            pause: None,
            stop: false,
            volume: None,
            speed: None,
        }
    }

    /// Pause the sound.
    pub fn pause(mut self) -> Self {
        self.pause = Some(true);
        self
    }

    /// Unpause the sound.
    pub fn unpause(mut self) -> Self {
        self.pause = Some(false);
        self
    }

    /// Pause or unpause the sound.
    pub fn set_paused(mut self, paused: bool) -> Self {
        self.pause = Some(paused);
        self
    }

    /// Stop and destroy the sound controller.
    pub fn stop(mut self) -> Self {
        self.stop = true;
        self
    }

    /// Set the volume level, typically `0.0`–`1.0`.
    pub fn set_volume(mut self, volume: f32) -> Self {
        self.volume = Some(volume);
        self
    }

    /// Set the playback speed multiplier; `1.0` is normal speed.
    pub fn set_speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }
}

fn is_false(b: &bool) -> bool {
    !b
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

static BOPPO_CA_CERT: &[u8] = include_bytes!("../assets/BoppoDeviceCA.crt");

/// The write half of a WebSocket connection to a Boppo tablet.
pub struct BoppoSender {
    sink: SplitSink<WsStream, Message>,
}

/// The read half of a WebSocket connection to a Boppo tablet.
pub struct BoppoReceiver {
    stream: SplitStream<WsStream>,
}

/// Connect to a Boppo tablet at `hostname`.
///
/// Hostname can be:
/// - A domain name (e.g. `boppo-<SERIAL_NUMBER>.local`)
/// - An IP address (e.g. `192.168.1.100`)
pub async fn connect(
    hostname: &str,
    password: &str,
) -> Result<(BoppoSender, BoppoReceiver), Error> {
    let url = format!("wss://{hostname}/ws");
    let mut root_store = RootCertStore::empty();
    let mut cert_bytes: &[u8] = BOPPO_CA_CERT;
    let certs = rustls_pemfile::certs(&mut cert_bytes)
        .collect::<Result<Vec<_>, _>>()
        .expect("BoppoDeviceCA.crt is embedded and must be valid PEM");
    for cert in certs {
        root_store
            .add(cert)
            .expect("BoppoDeviceCA.crt must be a valid certificate");
    }
    let tls = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = Connector::Rustls(Arc::new(tls));
    let mut request = url.into_client_request()?;
    request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {password}").parse().unwrap(),
    );
    let (ws, _) = connect_async_tls_with_config(request, None, false, Some(connector)).await?;
    let (sink, stream) = ws.split();
    Ok((BoppoSender { sink }, BoppoReceiver { stream }))
}

impl BoppoSender {
    /// Set all 40 lights. Each entry is the RGB color for that light, ordered
    /// as 4 lights per button (top, left, right, bottom) for buttons 0–9.
    pub async fn set_lights(&mut self, framebuffer: Framebuffer) -> Result<(), Error> {
        let mut data = Vec::with_capacity(132);
        data.extend_from_slice(b"set_lights ");
        for light in framebuffer.colors {
            data.extend_from_slice(&[light.r, light.g, light.b]);
        }
        self.sink.send(Message::Binary(data)).await?;
        Ok(())
    }

    /// Play a sound on the tablet.
    pub async fn play_sound(&mut self, si: &SoundInstruction) -> Result<(), Error> {
        let Some(ids) = si.controller_ids() else {
            return Err(crate::Error::InvalidMessage(
                "controller found inside Repeat".into(),
            ));
        };
        let json = serde_json::to_string(&si)?;
        let data = format!("play_sound {json}");
        self.sink.send(Message::Text(data)).await?;
        for id in ids {
            boppo_core::internal::on_sound_controller_started_playing(id);
        }
        Ok(())
    }

    /// Modify parameters of a currently-playing sound controlled by a
    /// Controller.
    pub async fn set_sound_parameter(
        &mut self,
        req: SetSoundParameterRequest,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(&req)?;
        let data = Message::Text(format!("set_sound_param {json}"));
        self.sink.send(data).await?;
        Ok(())
    }

    /// Stop all currently-playing sounds.
    pub async fn stop_all_sounds(&mut self) -> Result<(), Error> {
        let data = Message::Text("stop_all_sounds".to_string());
        self.sink.send(data).await?;
        Ok(())
    }

    /// Execute a shell command on the tablet.
    pub async fn execute_command(&mut self, command: &str) -> Result<(), Error> {
        let data = Message::Text(format!("execute_command {command}"));
        self.sink.send(data).await?;
        Ok(())
    }

    /// Close the WebSocket connection gracefully.
    pub async fn close(&mut self) -> Result<(), Error> {
        self.sink.close().await?;
        Ok(())
    }
}

/// An event received from a Boppo tablet.
#[non_exhaustive]
pub enum TabletEvent {
    /// A button was pressed or released.
    Button(ButtonEvent),
    /// A sound controller finished playing. Contains the controller ID.
    SoundFinished(u64),
    /// The tablet reported an error. Contains the error message.
    ErrorMessage(String),
}

impl BoppoReceiver {
    /// Wait for the next event from the tablet. Returns `None` when the
    /// connection is closed.
    pub async fn next_event(&mut self) -> Result<Option<TabletEvent>, Error> {
        loop {
            let Some(message) = self.stream.next().await else {
                return Ok(None);
            };
            let message = message.map_err(|e| Error::WebSocket(Box::new(e)))?;
            match message {
                Message::Text(text) => {
                    let Some((command, data)) = text.split_once(' ') else {
                        error!("received invalid message from tablet: {}", text);
                        continue;
                    };
                    match command {
                        "button" => {
                            return Ok(Some(parse_button_event_data(data)?));
                        }
                        "sound_finished" => {
                            return Ok(Some(parse_sound_finished_data(data)?));
                        }
                        "error_message" => {
                            return Ok(Some(TabletEvent::ErrorMessage(data.to_owned())));
                        }
                        _ => {
                            // do not error for forwards compatibility
                            debug!("received unknown command from tablet: {}", command);
                        }
                    }
                }
                Message::Close(_) => return Ok(None),
                // tungstenite automatically queues a Pong reply when a Ping is received,
                // flushed on the next send — no manual reply needed.
                Message::Ping(_) => {}
                Message::Binary(_) => {
                    // do not error for forwards compatibility
                    debug!("binary message received");
                }
                Message::Pong(_) => {}
                Message::Frame(_) => unreachable!("documented to not be returned"),
            }
        }
    }
}

fn parse_button_event_data(text: &str) -> Result<TabletEvent, Error> {
    let not_enough_parts = || Error::InvalidMessage("not enough parts in button message".into());
    let mut parts = text.split(' ');
    let index: usize = parts
        .next()
        .ok_or(not_enough_parts())?
        .parse()
        .map_err(|e: ParseIntError| Error::InvalidMessage(e.into()))?;
    if index >= Button::COUNT {
        return Err(Error::InvalidMessage("button index out of range".into()));
    }
    let button = Button::from_index(index);
    let pressed = match parts.next().ok_or(not_enough_parts())? {
        "p" => true,
        "r" => false,
        _ => return Err(Error::InvalidMessage("button pressed state invalid".into())),
    };
    let pressed_bits =
        u16::from_str_radix(parts.next().ok_or(not_enough_parts())?, 16).map_err(|e| {
            Error::InvalidMessage(format!("button pressed bits not a hex bitset: {e:?}").into())
        })?;
    let currently_pressed = Buttons::from_bitset(pressed_bits);
    Ok(TabletEvent::Button(ButtonEvent::new(
        button,
        pressed,
        currently_pressed,
    )))
}

fn parse_sound_finished_data(text: &str) -> Result<TabletEvent, Error> {
    let id: u64 = text
        .parse()
        .map_err(|e| Error::InvalidMessage(format!("error parsing id: {e:?}").into()))?;
    Ok(TabletEvent::SoundFinished(id))
}
