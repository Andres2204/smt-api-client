extern crate alloc; // Necesario para alloc::format

use core::num::NonZero;
use core::net::SocketAddrV4;
use defmt::{info, error, warn};
use embassy_executor::task;
use embassy_net::Stack;
use embassy_net::tcp::TcpSocket as NetTcpSocket;
use embassy_sync::pubsub::{DynPublisher, DynSubscriber};
use embassy_time::{Duration, Timer};

use embedded_tls::{TlsConfig, TlsConnection, TlsContext, UnsecureProvider};
use embedded_tls::Aes128GcmSha256;
use heapless::{String, Vec};
use rust_mqtt::client::{Client, options::{ConnectOptions, PublicationOptions, SubscriptionOptions}};
use rust_mqtt::types::{MqttBinary, MqttString, TopicName};
use rust_mqtt::buffer::AllocBuffer;
use rust_mqtt::client::event::Event;
use rust_mqtt::client::options::TopicReference;
use rust_mqtt::config::KeepAlive;
use crate::events::{Actuators, Command, Measurements, SENSOR_CH_CAP};

// BROKER
const BROKER_IP: &str = "192.168.58.176"; // broker.emqx.io TODO: impl dns if using domain name
const BROKER_PORT: u16 = 8883;

// MQTT TOPICS
const TOPIC_PREFIX: &str = "smartpot/v1/";

#[task]
pub async fn mqtt_task(stack: Stack<'static>, rng: esp_hal::rng::Trng, mut sensor_channel: DynSubscriber<'static, Measurements>, commands_channel: DynPublisher<'static, Command>) {
    stack.wait_config_up().await;

    let mut rx_buffer = [0u8; 4096];
    let mut tx_buffer = [0u8; 4096];

    let mut tls_rx = [0u8; 16640];
    let mut tls_tx = [0u8; 16640];

    loop {
        if let Some(config) = stack.config_v4() {
            info!("[MQTT] My IP: {}, MAC: {}, Gateway: {:?}", config.address, stack.hardware_address(), config.gateway);
        }

        info!("[MQTT] Intentando conectar a {}:{}", BROKER_IP, BROKER_PORT);

        let mut socket = NetTcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        let broker_addr: SocketAddrV4 = alloc::format!("{}:{}", BROKER_IP, BROKER_PORT)
            .parse()
            .expect("Dirección inválida");

        if let Err(e) = socket.connect(broker_addr).await {
            error!("[MQTT] TCP connect falló: {:?}", e);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }
        info!("[MQTT] TCP conectado");

        // TLS
        let tls_config: TlsConfig<> = TlsConfig::new()
            .with_server_name(BROKER_IP)
            .enable_rsa_signatures();
        let tls_context = TlsContext::new(&tls_config, UnsecureProvider::new::<Aes128GcmSha256>(rng.clone()));
        let mut tls = TlsConnection::new(socket, &mut tls_rx, &mut tls_tx);
        if let Err(e) = tls.open(tls_context).await {
            error!("[MQTT] TLS Error: {:?}", e);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }
        info!("[MQTT] TLS Connected");

        //  MQTT Client
        let mut buffer_provider = AllocBuffer;
        let mut client: Client<'_, &mut TlsConnection<NetTcpSocket, _>, _, 4, 4, 4, 4> = Client::new(&mut buffer_provider);
        let connect_options = ConnectOptions::new()
            .clean_start()          // ← sin argumento (true por defecto)
            .keep_alive(KeepAlive::Seconds(NonZero::new(60).unwrap()))
            .user_name(MqttString::from_str("esp32-smartpot").unwrap())
            .password(MqttBinary::from_slice(b"password").unwrap());

        if let Err(e) = client.connect(
            &mut tls,
            &connect_options,
            Some(MqttString::from_str("esp32-smartpot-v1").unwrap()), // client id
        ).await {
            error!("[MQTT] MQTT connection failed: {:?}", e);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }

        info!("[MQTT] MQTT connected successfully!");

        // Suscribirse
        let topic_base = alloc::format!(
            "{}{}/",
            TOPIC_PREFIX,
            stack.hardware_address()
        );
        let bind = alloc::format!("{}commands", topic_base);
        let topic_commands = TopicName::new(
            MqttString::from_str(&bind).unwrap()
        ).unwrap();

        if let Err(e) = client.subscribe(
            topic_commands.as_borrowed().into(),   // TopicFilter
            SubscriptionOptions::new().at_least_once(),
        ).await {
            error!("[MQTT] Subscription failed: {:?}", e);
        } else {
            info!("[MQTT] Successfully subscribe to {}", topic_commands);
        }

        loop {
            // Poll (mantener conexión viva y recibir mensajes)
            let poll = client.poll().await;
            if let Ok(event) = poll {
                match &event {
                    Event::Publish(publication) => {
                        let topic = &publication.topic;
                        let message = publication.message.as_bytes();
                        let msg_str = str::from_utf8(message).unwrap();

                        info!("[MQTT SUB] from {}: {}", topic, msg_str);
                        commands_channel.publish_immediate(Command::Activate(Actuators::Humidifier));
                        // send commands trough command_channel (publish_inmediate())
                    }
                    _ => {
                        info!("[MQTT] received event {}", event);
                    }
                }
            } else{
                error!("[MQTT] Poll error: {:?}", poll);
                break;
            }

            //
            //  PUBLISH SENSORS MEASUREMENTS
            //

            // TODO: This code is copied from telemetry task in wifi.rs
            // TODO: wrap in a feature
            let mut measures: Vec<Measurements, SENSOR_CH_CAP> = Vec::new();
            while let Some(m) = sensor_channel.try_next_message_pure() {
                let _ = measures.push(m);
            }

            let topic_reference = TopicReference::Name(topic_commands.clone());
            let pub_options = PublicationOptions::new(topic_reference);

            for m in &measures {
                let mut payload: String<255> = String::new();
                m.to_json(&mut payload);

                if let Err(e) = client.publish(
                    &pub_options,
                    rust_mqtt::Bytes::Borrowed(payload.as_bytes()),
                ).await {
                    error!("[MQTT] Publish falló: {:?}", e);
                    break;
                } else {
                    info!("[MQTT] Publicado: {}", payload.as_str());
                }
            }

            Timer::after(Duration::from_secs(2)).await;
        }

        client.abort().await;
        warn!("[MQTT] Conexión perdida. Reintentando en 5s...");
        Timer::after(Duration::from_secs(5)).await;
    }
}