//! This is a platform agnostic Rust driver for the VEML6075 UVA and UVB
//! light sensor, based on the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://github.com/rust-embedded/embedded-hal
//!
//! This driver allows you to:
//! - Enable/disable the sensor. See `enable()`.
//! - Read the UVA measurement. See `read_uva()`.
//! - Read the UVB measurement. See `read_uvb()`.
//! - Read the UVcomp1 measurement. See `read_uvcomp1()`.
//! - Read the UVcomp2 measurement. See `read_uvcomp2()`.
//! - Read all sensor data at once. See `read_all()`.
//! - Read the device id. See `read_device_id()`.
//!
//! ## The device
//! The VEML6075 senses UVA and UVB light and incorporates photodiode,
//! amplifiers,and analog / digital circuits into a single chip using a
//! CMOS process. When the UV sensor is applied, it is able to detect
//! UVA and UVB intensity to provide a measure of the signal strength
//! as well as allowing for UVI measurement.
//! The VEML6075 provides excellent temperature compensation capability
//! for keeping the output stable under changing temperature.
//! VEML6075's functionality is easilyoperated via the simple command
//! format of I2C (SMBus compatible) interface protocol.
//! VEML6075's operating voltage ranges from 1.7 V to 3.6 V.
//!
//! Datasheet:
//! - [VEML6075](https://www.vishay.com/docs/84304/veml6075.pdf)
//!
//! Application note:
//! - [VEML6075 AN](https://www.vishay.com/docs/84339/designingveml6075.pdf)
//!
//! ## Usage examples (see also examples folder)
//!
//! ### Read UVA and UVB
//!
//! Import this crate and an `embedded_hal` implementation, then instantiate
//! the device:
//!
//! ```no_run
//! extern crate linux_embedded_hal as hal;
//! extern crate veml6075;
//! use veml6075::Veml6075;
//!
//! # fn main() {
//! let dev = hal::I2cdev::new("/dev/i2c-1").unwrap();
//! let mut sensor = Veml6075::new(dev);
//! let uva = sensor.read_uva().unwrap();
//! let uvb = sensor.read_uvb().unwrap();
//! println!("Measurements UVA: {}, UVB: {}", uva, uvb);
//! # }
//! ```
//!
//! ### Read all channels at once
//!
//! ```no_run
//! extern crate linux_embedded_hal as hal;
//! extern crate veml6075;
//! use veml6075::Veml6075;
//!
//! # fn main() {
//! let dev = hal::I2cdev::new("/dev/i2c-1").unwrap();
//! let mut sensor = Veml6075::new(dev);
//! let measurement = sensor.read_all().unwrap();
//! println!("Measurements UVA: {}, UVB: {}", measurement.uva, measurement.uvb);
//! # }
//! ```

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![no_std]

extern crate embedded_hal as hal;
use hal::blocking::i2c::Write;

/// All possible errors in this crate
#[derive(Debug)]
pub enum Error<E> {
    /// I²C bus error
    I2C(E),
}

/// Result of measurement of all channels
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    /// UVA sensor data
    pub uva: u16,
    /// UVB sensor data
    pub uvb: u16,
    /// UVcomp1 sensor data
    pub uvcomp1: u16,
    /// UVcomp2 sensor data
    pub uvcomp2: u16,
}

struct Register;

impl Register {
    const CONFIG: u8 = 0x00;
    const UVA: u8 = 0x07;
    const UVB: u8 = 0x09;
    const UVCOMP1: u8 = 0x0A;
    const UVCOMP2: u8 = 0x0B;
    const DEVICE_ID: u8 = 0x0C;
}

struct BitFlags;

impl BitFlags {
    const SHUTDOWN: u8 = 0b0000_0001;
}

const DEVICE_ADDRESS: u8 = 0x10;

/// Integration durations for the UV measurement.
pub enum IntegrationTime {
    /// 50ms
    It50  = 0b000,
    /// 100ms
    It100 = 0b001,
    /// 200ms
    It200 = 0b010,
    /// 400ms
    It400 = 0b011,
    /// 800ms
    It800 = 0b100,
}

/// Veml6075 device driver.
#[derive(Debug, Default)]
pub struct Veml6075<I2C> {
    /// The concrete I²C device implementation.
    i2c: I2C,
    /// Configuration register status.
    config: u8,
}

impl<I2C, E> Veml6075<I2C>
where
    I2C: Write<Error = E>,
{
    /// Create new instance of the Veml6075 device.
    pub fn new(i2c: I2C) -> Self {
        Veml6075 {
            i2c,
            config: 0x01, // shutdown
        }
    }

    /// Destroy driver instance, return I²C bus instance.
    pub fn destroy(self) -> I2C {
        self.i2c
    }

    /// Enable the sensor.
    pub fn enable(&mut self) -> Result<(), Error<E>> {
        let config = self.config;
        self.write_config(config & !BitFlags::SHUTDOWN)
    }

    /// Disable the sensor (shutdown).
    pub fn disable(&mut self) -> Result<(), Error<E>> {
        let config = self.config;
        self.write_config(config | BitFlags::SHUTDOWN)
    }

    /// Set the integration time for measurements.
    pub fn set_integration_time(&mut self, it: IntegrationTime) -> Result<(), Error<E>> {
        let config = self.config;
        self.write_config(config | ((it as u8) << 4))
    }

    fn write_config(&mut self, config: u8) -> Result<(), Error<E>> {
        self.i2c
            .write(DEVICE_ADDRESS, &[Register::CONFIG, config, 0])
            .map_err(Error::I2C)?;
        self.config = config;
        Ok(())
    }
}

impl<I2C, E> Veml6075<I2C>
where
    I2C: hal::blocking::i2c::WriteRead<Error = E>,
{
    /// Read the sensor data of all channels at once.
    pub fn read_all(&mut self) -> Result<Measurement, Error<E>> {
        Ok(Measurement {
            uva: self.read_uva()?,
            uvb: self.read_uvb()?,
            uvcomp1: self.read_uvcomp1()?,
            uvcomp2: self.read_uvcomp2()?,
        })
    }

    /// Read the UVA sensor data.
    pub fn read_uva(&mut self) -> Result<u16, Error<E>> {
        self.read_register(Register::UVA)
    }

    /// Read the UVB sensor data.
    pub fn read_uvb(&mut self) -> Result<u16, Error<E>> {
        self.read_register(Register::UVB)
    }

    /// Read the UVcomp1 sensor data.
    pub fn read_uvcomp1(&mut self) -> Result<u16, Error<E>> {
        self.read_register(Register::UVCOMP1)
    }

    /// Read the UVcomp2 sensor data.
    pub fn read_uvcomp2(&mut self) -> Result<u16, Error<E>> {
        self.read_register(Register::UVCOMP2)
    }

    /// Read the device ID
    pub fn read_device_id(&mut self) -> Result<u16, Error<E>> {
        self.read_register(Register::DEVICE_ID)
    }

    fn read_register(&mut self, register: u8) -> Result<u16, Error<E>> {
        let mut data = [0; 2];
        self.i2c
            .write_read(DEVICE_ADDRESS, &[register], &mut data)
            .map_err(Error::I2C)?;
        Ok(u16::from(data[1]) << 8 | u16::from(data[0]))
    }
}

/*
  UV Index function based on code from the Arduino_MKRENV library.

  Copyright (c) 2019 Arduino SA. All rights reserved.

  This library is free software; you can redistribute it and/or
  modify it under the terms of the GNU Lesser General Public
  License as published by the Free Software Foundation; either
  version 2.1 of the License, or (at your option) any later version.

  This library is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
  Lesser General Public License for more details.

  You should have received a copy of the GNU Lesser General Public
  License along with this library; if not, write to the Free Software
  Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
*/

/// Convert the raw sensor readings into a UV Index.
///
/// Coefficients chosen from Designing the VEML6075 Into an Application
/// table UV COEFFICIENTS AND RESPONSIVITY for Open Air.
pub fn calculate_uv_index(uva: u16, uvb: u16, uvcomp1: u16, uvcomp2: u16) -> f32 {
    let uva_i: f32 = {
        let a: f32 = 2.22;
        let b: f32 = 1.33;

        (uva as f32) - (a * (uvcomp1 as f32)) - (b * (uvcomp2 as f32))
    };
    let uvb_i: f32 = {
        let c: f32 = 2.95;
        let d: f32 = 1.74;

        (uvb as f32) - (c * (uvcomp1 as f32)) - (d * (uvcomp2 as f32))
    };

    let index: f32 = {
        let uvaresp: f32 = 0.001461;
        let uvbresp: f32 = 0.002591;

        ((uva_i * uvaresp) + (uvb_i * uvbresp)) / 2.0
    };

    index
}
