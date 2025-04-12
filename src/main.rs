use crate::{cli::CliArgs, config::AppConfig, notify::notify_flood};

use clap::Parser;
use log::{debug, error, info, warn};
use rumqttc::{Client, Event::Incoming, MqttOptions, Packet::Publish, QoS};
use std::{process::exit, thread, time::Duration};

mod cli;
mod config;
mod notify;

fn main() {
    let cli = CliArgs::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbosity.into())
        .init();

    let config = match AppConfig::from_file(&cli.config) {
        Ok(config) => config,
        Err(err) => {
            error!("Could not load config: {err}");
            exit(1);
        }
    };

    debug!("Monitors: {monitors:?}", monitors = config.monitors);

    let mut opts = MqttOptions::new(
        config.mqtt.client_id,
        config.mqtt.hostname,
        config.mqtt.port,
    );
    opts.set_credentials(config.mqtt.username, config.mqtt.password);
    opts.set_keep_alive(Duration::from_secs(30));

    let (client, mut connection) = Client::new(opts, 10);

    // Iterate to poll the eventloop for connection progress
    for notification in connection.iter() {
        match notification {
            Err(err) => {
                warn!("MQTT error: {err:?}");
                thread::sleep(Duration::from_secs(2));
            }
            Ok(event) => match event {
                Incoming(rumqttc::Packet::ConnAck(_)) => {
                    info!("Connected; subscribing to monitored topics");
                    for topic in &config.monitors.flood_topics {
                        client
                            .subscribe(topic, QoS::AtMostOnce)
                            .expect("Subscribe failed");
                    }
                }
                Incoming(Publish(msg)) => {
                    let payload = String::from_utf8_lossy(&msg.payload);
                    debug!("MQTT publish: {topic} -> {payload}", topic = msg.topic);
                    if config.monitors.is_flood_topic(&msg.topic) && payload == "true" {
                        notify_flood(&config.pushover, &msg.topic);
                    }
                }
                _ => debug!("MQTT event: {event:?}"),
            },
        }
    }
}
