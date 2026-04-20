#![allow(unused)]   // quita warnings molestos mientras desarrollas

extern crate alloc; // Necesario para alloc::format

use core::num::NonZero;
use core::net::SocketAddrV4;
use defmt::{info, error, warn};
use embassy_executor::task;
use embassy_net::Stack;
use embassy_net::tcp::TcpSocket as NetTcpSocket;
use embassy_time::{Duration, Timer};

use rust_mqtt::client::{Client, options::{ConnectOptions, PublicationOptions, SubscriptionOptions}};
use rust_mqtt::types::{MqttString, TopicName};
use rust_mqtt::buffer::AllocBuffer;
use rust_mqtt::client::event::Event;
use rust_mqtt::client::options::TopicReference;
use rust_mqtt::config::KeepAlive;

const BROKER_IP: &str = "192.168.58.176";
const BROKER_PORT: u16 = 1883;

#[task]
pub async fn mqtt_task(stack: Stack<'static>) {

    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("[MQTT] My IP: {}, MAC: {}, Gateway: {:?}",
              config.address, stack.hardware_address(), config.gateway);
    }

    let mut rx_buffer = [0u8; 4096];
    let mut tx_buffer = [0u8; 4096];

    loop {
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

        // Cliente MQTT
        let mut buffer_provider = AllocBuffer;
        let mut client: Client<'_, &mut NetTcpSocket, _, 4, 4, 4, 4> = Client::new(&mut buffer_provider);

        // Opciones de conexión
        let connect_options = ConnectOptions::new()
            .clean_start()          // ← sin argumento (true por defecto)
            .keep_alive(KeepAlive::Seconds(NonZero::new(60).unwrap()));     // ← toma u16, no KeepAlive wrapper

        // Conexión MQTT (Client ID se pasa aquí)
        if let Err(e) = client.connect(
            &mut socket,
            &connect_options,
            Some(MqttString::from_str("esp32-smt-client").unwrap()),
        ).await {
            error!("[MQTT] MQTT CONNECT falló: {:?}", e);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }

        info!("[MQTT] ¡Conectado a MQTT correctamente!");

        // Suscribirse
        let topic_str = MqttString::from_str("test/topic").unwrap();
        let topic_name = TopicName::new(topic_str).unwrap();

        if let Err(e) = client.subscribe(
            topic_name.as_borrowed().into(),   // TopicFilter
            SubscriptionOptions::new().at_least_once(),
        ).await {
            error!("[MQTT] Subscribe falló: {:?}", e);
        } else {
            info!("[MQTT] Suscrito a test/topic");
        }

        // Loop principal
        let mut counter: u32 = 0;

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
                    }
                    _ => {
                        info!("[MQTT] received event {}", event);
                    }
                }
            } else{
                error!("[MQTT] Poll error: {:?}", poll);
                break;
            }

            // Publicar cada ~10 segundos
            let payload = alloc::format!("Hola desde ESP32 #{}", counter);

            let topic_reference = TopicReference::Name(topic_name.clone());
            let pub_options = PublicationOptions::new(
               topic_reference   // TopicReference
            );

            if let Err(e) = client.publish(
                &pub_options,
                rust_mqtt::Bytes::Borrowed(payload.as_bytes()),
            ).await {
                error!("[MQTT] Publish falló: {:?}", e);
                break;
            } else {
                info!("[MQTT] Publicado: {}", payload.as_str());  // defmt prefiere &str
            }

            counter += 1;
            Timer::after(Duration::from_secs(2)).await;
        }

        warn!("[MQTT] Conexión perdida. Reintentando en 5s...");
        Timer::after(Duration::from_secs(5)).await;
    }
}
