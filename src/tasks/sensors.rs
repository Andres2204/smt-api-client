use defmt::{info, error, debug};
use esp_hal::{i2c::master::I2c, Async};
use embassy_executor::task;
use embassy_time::{Timer, Duration};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex};
use embassy_sync::pubsub::DynPublisher;

use crate::events::Measurements;
use bh1750_embedded::r#async::Bh1750Async;
use bh1750_embedded::Resolution;
use embedded_hal_async::i2c::ErrorType;
use crate::drivers::bme280::MeasurementsBME280;
use crate::drivers::tca9548a::TcaChannel;

// TODO: dynamic recon of devices
#[task]
pub async fn bme280_task(i2c_bus: I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>, sensor_channel: DynPublisher<'static, Measurements>, address: u8 ) {
    bme280(i2c_bus, sensor_channel, address, None).await;
}

#[task]
pub async fn bme280_task_tca(i2c_bus: TcaChannel<I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>>, sensor_channel: DynPublisher<'static, Measurements>, address: u8 ) {
    let channel = Some(i2c_bus.get_channel());
    bme280(i2c_bus, sensor_channel, address, channel).await;
}

async fn bme280<I2C>(
    i2c_bus: I2C,
    sensor_channel: DynPublisher<'static, Measurements>,
    address: u8,
    channel: Option<u8>
)
where I2C: embedded_hal_async::i2c::I2c
{
    info!("Starting BME280 sensor task...");
    let mut bme280 = crate::drivers::bme280::Bme280::new(i2c_bus, address).await.unwrap();

    loop {
        if let Some(c) = channel {
            info!("Measuring BME280 sensor from channel {}", channel);
        } else {
            info!("Measuring BME280 sensor on default");
        }

        match bme280.measure().await {
            Ok(m) => {
                info!("Sending BME280:{} measurementes {}", channel, &m);
                sensor_channel.publish(Measurements::BME280((m.temperature, m.humidity, m.pressure))).await;}
            Err(e) => {
                error!("Error measuring BME280:{} sensor on {}", channel, address);
            }
        }

        Timer::after(Duration::from_secs(2)).await;
    }
}


#[task]
pub async fn bh1750_task(i2c_bus: I2cDevice<'static, NoopRawMutex, I2c<'static, Async>>, sensor_channel: DynPublisher<'static, Measurements>) {
    info!("Starting BM1750 sensor task...");
    let delay = embassy_time::Delay;
    let mut sensor = Bh1750Async::new(i2c_bus, delay, bh1750_embedded::Address::Low);

    loop {
        debug!("Measuring BH1750 sensor");
        let lux: f32 = match sensor.one_time_measurement(Resolution::High).await {
            Ok(measure) => { measure }
            Err(_e) => { error!("Measuring BH1750 sensor:"); -1.0}
        };

        debug!("Sending BH1750 lux measure {}", lux);
        sensor_channel.publish(Measurements::BH1750(lux)).await;

        Timer::after(Duration::from_secs(2)).await;
    }
}