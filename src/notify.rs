use crate::config::PushoverConfig;

use log::{debug, error, info};
use pushover::requests::message::SendMessage;
use pushover::{API, Priority, Sound};

pub fn notify_flood(config: &PushoverConfig, topic: &str) {
    info!("Notifying flood event ({topic})");
    let body = format!("Flood event on {topic}");

    let api = API::new();
    let mut msg = SendMessage::new(&config.token, &config.user, &body);
    msg.set_sound(Sound::Siren);
    msg.set_priority(Priority::High);

    match api.send(&msg) {
        Ok(resp) => {
            debug!("Notification sent: {resp:?}",)
        }
        Err(err) => {
            error!("Could not send push notification: {err}");
        }
    }
}
