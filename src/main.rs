use crate::{
    cli::CliArgs,
    config::{AppConfig, MQTTConfig},
    notify::{notify_flood, notify_freemdu, notify_mail},
};

use clap::Parser;
use log::{debug, error, info, warn};
use rumqttc::{AsyncClient, Event::Incoming, EventLoop, MqttOptions, Packet::Publish, QoS};
use serde_json::Value;
use std::{process::exit, time::Duration};
use tokio::{task::JoinSet, time};
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

    let mut tasks = JoinSet::new();
    if let Some(mqtt_local_config) = config.mqtt.local {
        let (local_client, local_eventloop) = make_mqtt_client(&mqtt_local_config);
        let topics: Vec<String> = match (config.flood.as_ref(), config.freemdu.as_ref()) {
            (Some(flood_config), None) => flood_config.topics.clone(),
            (None, Some(freemdu_config)) => vec![freemdu_config.topic.clone()],
            (Some(flood_config), Some(freemdu_config)) => {
                let mut topics = flood_config.topics.clone();
                topics.push(freemdu_config.topic.clone());
                topics
            }
            (None, None) => {
                panic!(
                    "Flood config + FreeMDU config missing when local MQTT connection is configured"
                );
            }
        };
        let pushover_config_local = config.pushover.clone();
        let mut freemdu_finished = false;
        tasks.spawn(mqtt_task(
            "local",
            topics,
            local_client,
            local_eventloop,
            move |topic, payload| {
                if let Some(ref flood_config) = config.flood
                    && flood_config.matches_topic(&topic)
                    && payload == "true"
                {
                    notify_flood(&pushover_config_local, &topic);
                }
                if let Some(ref freemdu_config) = config.freemdu
                    && freemdu_config.matches_topic(&topic)
                {
                    if payload == "ProgramFinished" && !freemdu_finished {
                        notify_freemdu(&pushover_config_local, &topic);
                        freemdu_finished = true;
                    } else if payload != "ProgramFinished" {
                        freemdu_finished = false;
                    }
                }
            },
        ));
    }

    if let Some(mqtt_ttn_config) = config.mqtt.ttn {
        let (ttn_client, ttn_eventloop) = make_mqtt_client(&mqtt_ttn_config);
        let mailbox_config = config
            .mailbox
            .expect("Mailbox config missing when TTN MQTT connection is configured");
        let pushover_config_ttn = config.pushover.clone();
        tasks.spawn(mqtt_task(
            "ttn",
            mailbox_config.topics.clone(),
            ttn_client,
            ttn_eventloop,
            move |topic: String, payload: String| {
                if !mailbox_config.matches_topic(&topic) {
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
                    }
                }
            },
        ));
    }

    // run forever since the tasks never terminate
    tasks.join_all().await;
}

fn make_mqtt_client(mqtt_config: &MQTTConfig) -> (AsyncClient, EventLoop) {
    let mut opts = MqttOptions::new(
        &mqtt_config.client_id,
        &mqtt_config.hostname,
        mqtt_config.port,
    );
    opts.set_credentials(&mqtt_config.username, &mqtt_config.password);
    opts.set_keep_alive(Duration::from_secs(30));

    AsyncClient::new(opts, 10)
}

async fn mqtt_task(
    prefix: &str,
    topics: Vec<String>,
    client: AsyncClient,
    mut eventloop: EventLoop,
    mut on_publish: impl FnMut(String, String) + Sync,
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
