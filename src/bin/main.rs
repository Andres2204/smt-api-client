#![no_std]
#![no_main]
#![deny(
clippy::mem_forget,
reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use esp_println::{self as _, println};

use static_cell::{ConstStaticCell, StaticCell};
use esp_rtos::embassy::Executor;
use esp_rtos::start_second_core;

use embassy_executor::Spawner;
use embassy_net::{DhcpConfig, StackResources};

use esp_hal::clock::CpuClock;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use esp_hal::{i2c::master::{Config, I2c}, Async};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::PubSubChannel;

use smt_api_client::tasks::wifi::{telemetry_task, net_task, wifi_connection_task};
use smt_api_client::events::{Measurements, SENSOR_CH};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("\n\n================ PANIC ================");
    if let Some(location) = info.location() {
        println!(
            "Panic in file '{}' at line {}",
            location.file(),
            location.line(),
        );
    } else {
        println!("Panic but no location information available.");
    }

    println!("message: {:?}", info.message());
    println!("======================================\n\n");

    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

pub const SSID: &str = "AQUI ESP32";
pub const PASSWORD: &str = "andres123";

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.1.0
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    info!("Embassy initialized!");

    //
    //  SECOND CORE STACK AND MAIN FUNCTION
    //
    // TODO: use feature to wrap it
    info!("Launching i2c sensor tasks");

    // I2C protocol pinout
    let i2c0 = peripherals.I2C0;
    let sda = peripherals.GPIO21;
    let scl = peripherals.GPIO22;

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let _software_interrupt = sw_int.software_interrupt2;
    let cpu1_main = move |spawner: Spawner| {
        let bme_pub = SENSOR_CH.dyn_publisher().unwrap();
        let bh_pub = SENSOR_CH.dyn_publisher().unwrap();
        static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2c<'static, Async>>> = StaticCell::new();
        let i2c = I2c::new(i2c0, Config::default()).unwrap().with_scl(scl).with_sda(sda).into_async();
        let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

        spawner.spawn(smt_api_client::tasks::sensors::bme280_task(I2cDevice::new(i2c_bus), bme_pub)).ok();
        spawner.spawn(smt_api_client::tasks::sensors::bh1750_task(I2cDevice::new(i2c_bus), bh_pub)).ok();
    };

    #[allow(static_mut_refs)]
    {
        const DISPLAY_STACK_SIZE: usize = 8192;
        static CORE1_STACK: StaticCell<esp_hal::system::Stack<DISPLAY_STACK_SIZE>> = StaticCell::new();
        let core1_stack = CORE1_STACK.init_with(|| { esp_hal::system::Stack::new() });
        start_second_core(
            peripherals.CPU_CTRL,
            sw_int.software_interrupt0,
            sw_int.software_interrupt1,
            core1_stack,
            || {
                info!("Starting second cpu (CORE1)");
                static EXECUTOR_CORE1: StaticCell<Executor> = StaticCell::new();
                let exec = EXECUTOR_CORE1.init_with(|| { Executor::new() });
                exec.run(cpu1_main);
            }
        );
    }

    //
    // WIFI INITIALIZATION
    //
    info!("Setting up network _stack");
    static WIFI_INIT: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let radio_init = WIFI_INIT.init_with(|| { esp_radio::init().expect("Failed to initialize radio controller") });

    let (wifi_controller, interfaces) = esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let wifi_interface = interfaces.sta;

    // init network _stack (with dhcp)
    static STACK_RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let stack_resources = STACK_RESOURCES.init_with(|| { StackResources::<5>::new() });
    let rng = Rng::new();
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
    let config = embassy_net::Config::dhcpv4(DhcpConfig::default());
    let (_stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        stack_resources,
        net_seed
    );

    //
    //  Spawn tasks (Telemetry and radio connections)
    //

    // Pub Sub Channel between telemtry and Communications tasks

    static DIVIDE_TO_EXTERIOR_CHANNEL: ConstStaticCell<PubSubChannel<CriticalSectionRawMutex, Measurements, 16, 4, 1>> = ConstStaticCell::new(PubSubChannel::new());
    let dtec = DIVIDE_TO_EXTERIOR_CHANNEL.take();

    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(wifi_connection_task(wifi_controller, SSID, PASSWORD)).ok();
    spawner.spawn(telemetry_task(SENSOR_CH.dyn_subscriber().unwrap(), dtec.dyn_publisher().unwrap())).ok();

    #[cfg(feature = "http-api")]
    spawner.spawn(smt_api_client::tasks::wifi::http_api_task(_stack, dtec.dyn_subscriber().unwrap())).ok();

    loop {
        //info!("Memory stats: {}", esp_alloc::HEAP.stats());
        //Timer::after(Duration::from_secs(5)).await;
        core::future::pending::<()>().await;
    }
}