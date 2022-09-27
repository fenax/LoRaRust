use core::convert::Infallible;

use bsp::hal::spi::Enabled;
use bsp::{entry, hal::gpio::FunctionSpi};
use defmt::export::panic;
use defmt::*;
use defmt_rtt as _;
use embedded_hal_compat::eh0_2::blocking::delay::DelayUs;
use embedded_hal_compat::eh0_2::digital::v2::{InputPin, OutputPin};
//use embedded_hal::spi::blocking::SpiDevice;
//{Transfer, Write};
use embedded_hal_compat::eh1_0::spi::blocking::{Transactional, TransferInplace, Write};
//use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal_compat::eh0_2::spi::{Mode, Phase, Polarity, MODE_0};
use embedded_hal_compat::{ForwardCompat, ReverseCompat};
use fugit::RateExtU32;
use numtoa::NumToA;
use panic_probe as _;

use radio_sx127x::base::HalError;
// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    spi::Spi,
    watchdog::Watchdog,
};

use radio_sx127x::{
    device::lora::{
        Bandwidth, CodingRate, FrequencyHopping, LoRaChannel, LoRaConfig, PayloadCrc,
        PayloadLength, SpreadingFactor,
    },
    device::{Channel, Modem, PaConfig, PaSelect},
    prelude::*, // prelude has Sx127x,
};
use radio_sx127x::{lora, Error as sx127xError}; // Error name conflict with hals

use radio::{Receive, Transmit};

use crate::input::Button2;

//use hal::spidev::{self, SpidevOptions};
//use hal::sysfs_gpio::Direction;
//use bsp::hal::Delay;
//use hal::{Pin, Spidev};

pub const MODE: Mode = Mode {
    //  SPI mode for radio
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

pub const FREQUENCY: u32 = 433_400_000; // frequency in hertz ch_12: 915_000_000, ch_2: 907_400_000

pub const CONFIG_CH: LoRaChannel = LoRaChannel {
    freq: FREQUENCY as u32, // frequency in hertz
    bw: Bandwidth::Bw125kHz,
    sf: SpreadingFactor::Sf7,
    cr: CodingRate::Cr4_8,
};

pub const CONFIG_LORA: LoRaConfig = LoRaConfig {
    preamble_len: 0x8,
    symbol_timeout: 0x64,
    payload_len: PayloadLength::Variable,
    payload_crc: PayloadCrc::Enabled,
    frequency_hop: FrequencyHopping::Disabled,
    invert_iq: false,
};

//   compare other settings in python version
//    lora.set_mode(sx127x_lora::RadioMode::Stdby).unwrap();
//    set_tx_power(level, output_pin) level >17 => PA_BOOST.
//    lora.set_tx_power(17,1).unwrap();
//    lora.set_tx_power(15,1).unwrap();

//baud = 1000000 is this needed for spi or just USART ?

pub const CONFIG_PA: PaConfig = PaConfig {
    output: PaSelect::Boost,
    power: 1,
};

//let CONFIG_RADIO = Config::default() ;

pub const CONFIG_RADIO: radio_sx127x::device::Config = radio_sx127x::device::Config {
    modem: Modem::LoRa(CONFIG_LORA),
    channel: Channel::LoRa(CONFIG_CH),
    pa_config: CONFIG_PA,
    xtal_freq: 32000000, // CHECK
    timeout_ms: 100,
};

pub enum State
//<T>
//where
//    T: core::fmt::Debug + 'static,
{
    Init,
    PrepareIdle,
    Reset,
    Idle,
    Sending,
    SendingDone,
    Received,
    Panic,
    //    Error(sx127xError<HalError<T, Infallible, Infallible>>),
}

pub enum Error<T>
where
    T: core::fmt::Debug + 'static,
{
    Radio(sx127xError<T>),
}

impl<T: core::fmt::Debug + 'static> From<sx127xError<T>> for Error<T> {
    fn from(e: sx127xError<T>) -> Self {
        Error::Radio(e)
    }
}

pub fn copy(src: &[u8], target: &mut [u8], cursor: &mut usize) {
    for c in src {
        if *cursor >= target.len() {
            return;
        }
        target[*cursor] = *c;
        *cursor += 1;
    }
}
