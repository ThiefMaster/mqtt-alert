use crate::{
    cli::CliArgs,
    config::{AppConfig, MQTTConfig},
    notify::{notify_flood, notify_mail},
};

use clap::Parser;
use log::{debug, error, info, warn};
use rumqttc::{AsyncClient, Event::Incoming, EventLoop, MqttOptions, Packet::Publish, QoS};
use serde_json::Value;
use std::{process::exit, time::Duration};
use tokio::{task, time};
mod cli;
mod config;
mod notify;

#[tokio::main(flavor = "current_thread")]
async fn main() {
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

    debug!("Flood: {flood:?}", flood = config.flood);
    debug!("Mailbox: {mailbox:?}", mailbox = config.mailbox);

    let (local_client, local_eventloop) = make_mqtt_client(&config.mqtt.local);
    let (ttn_client, ttn_eventloop) = make_mqtt_client(&config.mqtt.ttn);

    let pushover_config_local = config.pushover.clone();
    let local_task = task::spawn(mqtt_task(
        "local",
        config.flood.topics.clone(),
        local_client,
        local_eventloop,
        move |topic, payload| {
            if config.flood.matches_topic(&topic) && payload == "true" {
                notify_flood(&pushover_config_local, &topic);
            }
        },
    ));

    let pushover_config_ttn = config.pushover.clone();
    let ttn_task = task::spawn(mqtt_task(
        "ttn",
        config.mailbox.topics.clone(),
        ttn_client,
        ttn_eventloop,
        move |topic: String, payload: String| {
            if !config.mailbox.matches_topic(&topic) {
                return;
            }
            match serde_json::from_str::<Value>(&payload) {
                Ok(value) => {
                    if value["uplink_message"]["decoded_payload"]["DOOR_OPEN_STATUS"] == 1 {
                        notify_mail(&pushover_config_ttn, &topic);
                    }
                }
                Err(err) => {
                    warn!("Could not parse door sensor payload: {err:?}");
                    return;
                }
            }
        },
    ));

    // run forever since the tasks never terminate
    let _ = tokio::join!(local_task, ttn_task);
}

fn make_mqtt_client(mqtt_config: &MQTTConfig) -> (AsyncClient, EventLoop) {
    let mut opts = MqttOptions::new(
        &mqtt_config.client_id,
        &mqtt_config.hostname,
        mqtt_config.port,
    );
    opts.set_credentials(&mqtt_config.username, &mqtt_config.password);
    opts.set_keep_alive(Duration::from_secs(30));

    return AsyncClient::new(opts, 10);
}

async fn mqtt_task(
    prefix: &str,
    topics: Vec<String>,
    client: AsyncClient,
    mut eventloop: EventLoop,
    on_publish: (impl Fn(String, String) + Sync),
) {
    loop {
        let event = eventloop.poll().await;
        match event {
            Err(err) => {
                warn!("{prefix}: MQTT error: {err:?}");
                time::sleep(Duration::from_secs(2)).await;
            }
            Ok(event) => match event {
                Incoming(rumqttc::Packet::ConnAck(_)) => {
                    info!("{prefix}: Connected; subscribing to monitored topics");
                    for topic in &topics {
                        client
                            .subscribe(topic, QoS::AtMostOnce)
                            .await
                            .expect("Subscribe failed");
                    }
                }
                Incoming(Publish(msg)) => {
                    let payload = String::from_utf8_lossy(&msg.payload);
                    debug!(
                        "{prefix}: MQTT publish: {topic} -> {payload}",
                        topic = msg.topic
                    );
                    on_publish(msg.topic.to_string(), payload.to_string());
                }
                _ => debug!("{prefix}: MQTT event: {event:?}"),
            },
        }
    }
}
