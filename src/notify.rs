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

pub fn notify_mail(config: &PushoverConfig, topic: &str) {
    info!("Notifying mail event ({topic})");
    let api = API::new();
    let msg = SendMessage::new(&config.token, &config.user, "Snailmail arrived");

    match api.send(&msg) {
        Ok(resp) => {
            debug!("Notification sent: {resp:?}",)
        }
        Err(err) => {
            error!("Could not send push notification: {err}");
        }
    }
}

pub fn notify_freemdu(config: &PushoverConfig, topic: &str) {
    info!("Notifying freemdu event ({topic})");
    let api = API::new();
    let msg = SendMessage::new(&config.token, &config.user, "Washing machine finished");

    match api.send(&msg) {
        Ok(resp) => {
            debug!("Notification sent: {resp:?}",)
        }
        Err(err) => {
            error!("Could not send push notification: {err}");
        }
    }
}

pub fn notify_temperature(
    config: &PushoverConfig,
    sensor_name: &str,
    temperature: f64,
    exceeded: bool,
) {
    info!("Notifying temperature event ({sensor_name}, {temperature}, {exceeded})");
    let api = API::new();
    let msg = if exceeded {
        let mut msg = SendMessage::new(
            &config.token,
            &config.user,
            format!("Temperature on {sensor_name} too high: {temperature} °C"),
        );
        msg.set_sound(Sound::Siren);
        msg.set_priority(Priority::High);
        msg
    } else {
        SendMessage::new(
            &config.token,
            &config.user,
            format!("Temperature on {sensor_name} OK again: {temperature} °C"),
        )
    };

    match api.send(&msg) {
        Ok(resp) => {
            debug!("Notification sent: {resp:?}",)
        }
        Err(err) => {
            error!("Could not send push notification: {err}");
        }
    }
}
