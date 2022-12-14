#![no_std]
#![no_main]
mod blink;
mod input;
mod stuff;

use bsp::{entry, hal::gpio::FunctionSpi};
use defmt::*;
use defmt_rtt as _;
use embedded_hal_compat::eh0_2::digital::v2::InputPin;
use embedded_hal_compat::eh0_2::spi::{Mode, Phase, Polarity};
use embedded_hal_compat::ForwardCompat;
use fugit::RateExtU32;
use panic_probe as _;
use stuff::*;

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

use radio_sx127x::Error as sx127xError;
use radio_sx127x::{
    device::lora::{
        Bandwidth, CodingRate, FrequencyHopping, LoRaChannel, LoRaConfig, PayloadCrc,
        PayloadLength, SpreadingFactor,
    },
    device::{Channel, Modem, PaConfig, PaSelect},
    prelude::*, // prelude has Sx127x,
}; // Error name conflict with hals

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

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let delay =
        cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz()).forward();

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut _led = pins.led.into_push_pull_output();
    //blink(&mut led, &"Hello!");

    let _miso = pins.gpio8.into_mode::<FunctionSpi>();
    let _mosi = pins.gpio11.into_mode::<FunctionSpi>();
    let _clk = pins.gpio10.into_mode::<FunctionSpi>();

    let spi = Spi::<_, _, 8>::new(pac.SPI1);
    let spi = spi
        .init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            20.MHz(),
            &embedded_hal_02::spi::MODE_0,
        )
        .forward();

    let cs = pins.gpio9.into_readable_output().forward();
    let reset = pins.gpio7.into_readable_output().forward();
    let busy = pins.gpio12.into_floating_input().forward();
    let ready = pins.gpio13.into_floating_input().forward();

    let mut lora = Sx127x::spi(spi, cs, busy, ready, reset, delay, &CONFIG_RADIO).unwrap();
    /*let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay).unwrap_or_else(|_x| {
        blink(&mut led, "module");
        crate::panic!("Failed to communicate with radio module!");
    });*/

    let message = "Hello, world!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }
    let mut _cursor = 0;
    let mut button = Button2::new(pins.gpio19.into_pull_up_input());
    let mut state = stuff::State::Init;

    loop {
        state = match state.run_state(&mut lora, &mut button) {
            Err(stuff::Error::Radio(e)) => match e {
                sx127xError::Hal(_) => crate::panic!("HAL problem"),
                sx127xError::InvalidConfiguration => crate::panic!("invalid Configuration"),
                sx127xError::Aborted => {
                    info!("Transaction aborted");
                    State::PrepareIdle
                }
                sx127xError::InvalidResponse => {
                    info!("Invalid response");
                    State::Reset
                }
                sx127xError::Timeout => {
                    info!("Timeout");
                    State::Reset
                }
                sx127xError::Crc => State::PrepareIdle,
                sx127xError::BufferSize => State::PrepareIdle,
                sx127xError::InvalidDevice(_) => {
                    info!("invalid device, restarting");
                    State::Reset
                }
            },
            Ok(state) => state,
        }
    }
}

use core::fmt::Debug;

impl stuff::State {
    fn run_state<Hal: radio_sx127x::base::Hal, P: InputPin, T: Debug + 'static>(
        &self,
        lora: &mut radio_sx127x::Sx127x<Hal>,
        button: &mut Button2<P>,
    ) -> Result<Self, stuff::Error<T>>
    where
        stuff::Error<T>: From<sx127xError<T>>,
        P::Error: Debug,
        stuff::Error<T>: From<radio_sx127x::Error<<Hal as radio_sx127x::base::Hal>::Error>>,
    {
        match self {
            State::Init => {
                info!("init");
                Ok(State::PrepareIdle)
            }
            State::Reset => {
                crate::panic!("reset unimplemented")
            }
            State::PrepareIdle => {
                lora.start_receive()?;

                Ok(State::Idle)
            }
            State::Idle => {
                if button.just_pressed() {
                    info!("Send packet");
                    lora.start_transmit(b"Kikooo\n UWU ")?;
                    Ok(State::Sending)
                } else {
                    match lora.check_receive(false)? {
                        true => Ok(State::Received), //have a valid packet in the buffer
                        false => Ok(State::Idle),    //got an invalid packet
                    }
                }
            }
            State::Sending => match lora.check_transmit()? {
                true => Ok(State::SendingDone),
                false => Ok(State::Sending),
            },
            State::Received => {
                let mut buff = [0u8; 256];
                let (len, info) = lora.get_received(&mut buff)?;
                info!(
                    "received packet len = {} info : {} {}",
                    len, info.rssi, info.snr
                );
                //Ok(Self::Idle)
                lora.start_transmit(&buff[..len])?;
                Ok(State::Sending)
            }
            State::SendingDone => {
                lora.start_receive()?;
                Ok(Self::Idle)
            }
        }
    }
}
