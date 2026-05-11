use defmt::Format;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use heapless::String;

// SENSORS
#[derive(Clone, Copy, Debug, Format)]
pub enum Measurements {
    BME280( (f32, f32, f32) ),
    BH1750( f32 ),
    PH( f32 ),
    TDS( f32 ),
}
unsafe impl Send for Measurements {}

impl Measurements {
    pub fn to_json<const N: usize>(&self, buf: &mut String<N>) {
        use core::fmt::Write;

        let _ = match self {
            Measurements::BME280((t, h, p)) => {
                write!(buf, "{{\"temperature\":{},\"humidity\":{},\"pressure\":{}}}", t, h, p)
            }
            Measurements::BH1750(l) => {
                write!(buf, "{{\"lux\":{}}}", l)
            }
            Measurements::PH(p) => {
                write!(buf, "{{\"ph\":{}}}", p)
            }
            Measurements::TDS(t) => {
                write!(buf, "{{\"tds\":{}}}", t)
            }
        };
    }
}

pub const SENSOR_CH_CAP: usize = 8;
pub const SENSOR_CH_PUB: usize = 4;
pub const SENSOR_CH_SUB: usize = 2;
pub static SENSOR_CH: PubSubChannel<CriticalSectionRawMutex, Measurements, SENSOR_CH_CAP, SENSOR_CH_SUB, SENSOR_CH_PUB> = PubSubChannel::new();

//
// COMMANDS / ACTUATORS
//

#[derive(Clone, Copy, Debug, Format)]
pub enum Actuators {
    WaterPump,
    Lights,
    Humidifier,
}

#[derive(Clone, Copy, Debug, Format)]
pub enum Command {
    Activate(Actuators),
    Disable(Actuators),
    Threshold(Measurements),
}

unsafe impl Send for Command {}

pub const COMMAND_CH_CAP: usize = 8;
pub const COMMAND_CH_PUB: usize = 1;
pub const COMMAND_CH_SUB: usize = 1;
pub static COMMAND_CH: PubSubChannel<CriticalSectionRawMutex, Command, COMMAND_CH_CAP, COMMAND_CH_SUB, COMMAND_CH_PUB> = PubSubChannel::new();

/* FUTURE PLANS
pub enum SystemServices {
    WifiUp(bool),
    BLE(bool)
}
*/
