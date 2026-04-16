/*
use defmt::{info, error};
use embassy_executor::task;
use embassy_net::Stack;
use embassy_net::tcp::TcpSocket;
use rust_mqtt::client::{
    Client,
    options::{ConnectOptions, SubscriptionOptions, PublicationOptions, DisconnectOptions},
    event::Event,
};
use rust_mqtt::config::{SessionExpiryInterval};
use rust_mqtt::types::{MqttBinary, MqttString, TopicName};
use rust_mqtt::session::CPublishFlightState;
use rust_mqtt::buffer::AllocBuffer;


#[task]
async fn run_mqtt(stack: Stack<'static>) {
    use embedded_io_async::{Read, Write};

    info!("Starting MQTT task");
    let mut buffer = AllocBuffer;
    let mut client = Client::new(&mut buffer);

    static mut RX_BUFFER: [u8; 4096] = [0; 4096];
    static mut TX_BUFFER: [u8; 4096] = [0; 4096];

    let socket = TcpSocket::new(
        stack,
        &mut RX_BUFFER,
        &mut TX_BUFFER,
    );

    let remote = "192.168.58.176:1883"; // tu endpoint
    if let Err(e) = socket.connect(remote).await {
        error!("Failed to connect: {:?}", e);
        return;
    }

    let transport = socket;

    let connect_options = ConnectOptions::new()
        .clean_start()
        .session_expiry_interval(SessionExpiryInterval::NeverEnd)
        .user_name(MqttString::from_str("user").unwrap())
        .password(MqttBinary::from_slice(b"pass").unwrap());

    client.connect(
        transport,
        &connect_options,
        Some(MqttString::from_str("Hydroponic").unwrap()),
    ).await.unwrap();

    let topic = TopicName::new(MqttString::from_str("demo/topic").unwrap()).unwrap();

    client.subscribe(
        topic.as_borrowed().into(),
        SubscriptionOptions::new().exactly_once(),
    ).await.unwrap();

    let packet_identifier = client.publish(
        &PublicationOptions::new(topic.as_borrowed().into()).exactly_once(),
        "Hello World!".into(),
    ).await.unwrap().unwrap();

    while let Ok(event) = client.poll().await {
        if let Event::PublishComplete(_) = event {
            // Publish succeeded, we can disconnect
            client.disconnect(&DisconnectOptions::new()).await.unwrap();
            return;
        }
    }

    // An error has occured (e.g. network failure)
    client.abort().await;

    let transport = socket;    // Open a fresh connection

    client.connect(
        transport,
        &connect_options,
        Some(MqttString::from_str("rust-mqtt-demo").unwrap()),
    ).await.unwrap();


    // Recover the in-flight Quality of Service 2 publish.

    match client.session().cpublish_flight_state(packet_identifier) {
        // - Republish if PUBLISH / PUBREC may have been lost
        Some(CPublishFlightState::AwaitingPubrec) => client.republish(
            packet_identifier,
            &PublicationOptions::new(topic.into()).exactly_once(),
            "Hello World!".into(),
        ).await.unwrap(),
        // - Re-release if PUBREL / PUBCOMP may have been lost
        Some(CPublishFlightState::AwaitingPubcomp) => client.rerelease().await.unwrap(),
        // - Flight state already completed
        _ => {}
    }
}

*/