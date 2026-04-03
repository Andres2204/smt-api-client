use defmt::Format;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

#[derive(Clone, Copy, Debug, Format)]
pub enum Measurements {
    BME280( (f32, f32, f32) ),
    BH1750( f32 ),
    PH( f32 ),
    TDS( f32 ),
}
unsafe impl Send for Measurements {}

pub const SENSOR_CH_CAP: usize = 8;
pub const SENSOR_CH_PUB: usize = 2;
pub const SENSOR_CH_SUB: usize = 1;
pub static SENSOR_CH: PubSubChannel<CriticalSectionRawMutex, Measurements, SENSOR_CH_CAP, SENSOR_CH_SUB, SENSOR_CH_PUB> = PubSubChannel::new();

/*
pub enum SystemServices {
    WifiUp(bool),
    BLE(bool)
}
*/
