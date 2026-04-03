use defmt::{info, warn, debug};
use esp_println::println;
use esp_hal::rng::Rng;
use esp_radio::wifi::{ModeConfig, WifiDevice, WifiMode};
use esp_radio::wifi::ScanConfig;
use esp_radio::wifi::ClientConfig;
use esp_radio::wifi::WifiStaState;
use esp_radio::wifi::WifiEvent;
use esp_radio::wifi::WifiController;
use embassy_net::{Runner, Stack};
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_time::{Duration, Timer, WithTimeout};
use embassy_sync::pubsub::DynSubscriber;
use heapless::Vec;
use reqwless::client::{HttpClient, TlsConfig};
use crate::events::{Measurements, SENSOR_CH_CAP};

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    info!("Starting network stack");
    runner.run().await
}

#[embassy_executor::task]
pub async fn wifi_connection_task(mut wifi: WifiController<'static>, ssid: &'static str, password: &'static str) -> ! {
    wifi.set_mode(WifiMode::Sta).unwrap();
    wifi.start_async().with_timeout(Duration::from_secs(3)).await.unwrap().unwrap();
    Timer::after(Duration::from_secs(2)).await;

    loop {
        info!("[WIFI] Connecting to wifi...");
        info!("[WIFI] Scanning for Wifi Networks");
        // todo: channel send state
        let networks = wifi.scan_with_config_async(ScanConfig::default()).await.unwrap();
        for n in networks {
            info!("wifi network: {} @ {}db", n.ssid.as_str(), n.signal_strength);
            /*
            if let Some(cfg) = config_for_network(&n.ssid) {
                wifi.set_config(&esp_radio::wifi::ModeConfig::Client(cfg)).unwrap();
                if wifi.connect_async().await.is_err() {
                    error!("Unable to connect to wifi {n:?}");
                }
                break;
            }
            */
        }

        // TODO. actualizar para soportar diferentes redes (como esta comentado arriba)
        let client_config = ModeConfig::Client(ClientConfig::default()
            .with_ssid(ssid.into())
            .with_password(password.into())
        );
        wifi.set_config(&client_config).unwrap();

        if wifi.connect_async().await.is_err() {
            warn!("Unable to connect to wifi {}", ssid);
        }

        match wifi.is_connected() {
            Ok(true) => {
                info!("[WIFI] Wifi is online!");
                // todo: channel send wifi::connected | up | etc
                wifi.wait_for_event(WifiEvent::StaDisconnected).await;
                warn!("[WIFI] Wifi disconnected");
            },
            Ok(false) => {
                warn!("[WIFI] Wifi offline");
                // todo: channel send wifi::Offline
                Timer::after(Duration::from_secs(5)).await;
            },
            Err(e) => {
                warn!("[WIFI] An error occurred: {}", e);
                Timer::after(Duration::from_secs(5)).await;
            }
        }
    }

}

#[embassy_executor::task]
pub async fn http_api_task(stack: Stack<'static>) {
    let seed = Rng::new().random() as u64;
    let mut rx_buf = [0; 4096];
    let mut tx_buf = [0; 4096];

    let dns = DnsSocket::new(stack);
    let tcp_state = TcpClientState::<1, 4096, 4096>::new();
    let tcp = TcpClient::new(stack, &tcp_state);
    let tls = TlsConfig::new(
        seed,
        &mut rx_buf,
        &mut tx_buf,
        reqwless::client::TlsVerify::None
    );

    let mut client = HttpClient::new_with_tls(&tcp, &dns, tls);
    loop {
        stack.wait_config_up().await;
        // todo: save ip (if let Some(config) = stack.config_v4() { config.address}

        make_request(&mut client).await;
        Timer::after(Duration::from_secs(10)).await;
    }
}

async fn make_request(client: &mut HttpClient<'_, TcpClient<'_, 1, 4096, 4096>, DnsSocket<'_>>) {
    info!("[WIFI REQUEST] Making a request");
    Timer::after(Duration::from_secs(1)).await;
}

#[embassy_executor::task]
pub async fn telemetry_task(mut sensors_channel: DynSubscriber<'static, Measurements>) {
    info!("Telemetry task started");
    loop {

        let mut measures: Vec<Measurements, SENSOR_CH_CAP> = Vec::new();
        while let Some(m) = sensors_channel.try_next_message_pure() {
            let _ = measures.push(m);
        }

        if !measures.is_empty() {
            debug!("Telemetry: Measures received: {}", measures.len());
            for m in measures {
                debug!("\t{:?}", m);
            }
        } else {
            debug!("No measurements found");
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}


#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>, ssid: &'static str, password: &'static str) -> ! {
    info!("Start connection task");
    //println!("Device capabilities: {:?}", controller.capabilities());

    loop {
        match esp_radio::wifi::sta_state() {
            WifiStaState::Connected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(ssid.into())
                    .with_password(password.into()),
            );
            controller.set_config(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!\nScanning...");

            let scan_config = ScanConfig::default().with_max(10);
            let result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();

            for ap in result {
                println!("{:?}", ap);
            }
        }

        println!("About to connect...");
        match controller.connect_async().await {
            Ok(_) => info!("Wifi connected!"),
            Err(e) => {
                warn!("Failed to connect to wifi {:?}", e);
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}
