# 🥬 SmartPot

## Note:
This project is an early version of [SmartPotTech](https://github.com/SmartPotTech) esp32 firmware.


SmartPot is an IoT system designed for plant monitoring and automation, built around an ESP32 that collects sensor data and communicates it via MQTT to other services. The project is designed with a modular architecture in Rust, using asynchronous tasks and channel-based communication to decouple data acquisition, connectivity, and command processing.

The device can send measurements such as temperature, humidity, or other environmental data to an MQTT broker, as well as receive remote commands to control actuators or modify the system’s behavior. Communication uses TLS to secure connections, and the project aims to implement its own low-level components, including embedded drivers and direct hardware management.

Overall, SmartPot functions as a smart monitoring platform for plant pots or crops, focused on reliability, connectivity, and remote control of embedded hardware.

## Tech Stack
* **Rust Programming Language**: The project is built using the Rust programming language, providing a safe and efficient development environment.
* **ESP32 Hardware**: The project is designed to work with ESP32 hardware, providing a robust and scalable platform for IoT applications.
* **MQTT Protocol**: The project uses the MQTT protocol for communication with smart home devices and sensors.
* **Embassy Framework**: The project leverages the Embassy framework, providing a set of libraries and tools for building embedded systems.
* **Defmt Logging**: The project uses defmt logging, providing a efficient and flexible logging system.
* **Heapless Allocator**: The project uses the heapless allocator, providing a memory-safe and efficient allocation system.

## Project Structure
```markdown
src
├── bin
│   └── main.rs
├── drivers
│   ├── bme280.rs
│   ├── mod.rs
│   └── tca9548a.rs
├── events.rs
├── lib.rs
└── tasks
    ├── command.rs
    ├── mod.rs
    ├── mqtt.rs
    ├── sensors.rs
    └── wifi.rs
Cargo.toml
```