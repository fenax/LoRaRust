#![no_std]
#![no_main]
mod blink;
mod input;

use core::convert::Infallible;

use blink::blink;
use bsp::{entry, hal::gpio::FunctionSpi};
use defmt::export::panic;

use defmt::*;
use defmt_rtt as _;
use embedded_hal_compat::eh0_2::blocking::delay::DelayUs;
use embedded_hal_compat::eh0_2::digital::v2::{InputPin, OutputPin};
use embedded_hal_compat::eh0_2::spi::{Mode, Phase, Polarity, MODE_0};
use embedded_hal_compat::eh1_0::spi::blocking::{Transactional, TransferInplace, Write};
use embedded_hal_compat::{ForwardCompat, ReverseCompat};
use fugit::RateExtU32;
use numtoa::NumToA;
use panic_probe as _;

use display_interface_spi::SPIInterface;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::*,
};
use radio_sx127x::base::HalError;

use input::*;
use mipidsi::*;
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

//use hal::spidev::{self, SpidevOptions};
//use hal::sysfs_gpio::Direction;
//use bsp::hal::Delay;
//use hal::{Pin, Spidev};

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

pub const MODE: Mode = Mode {
    //  SPI mode for radio
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

pub const FREQUENCY: u32 = 433_400_000; // frequency in hertz ch_12: 915_000_000, ch_2: 907_400_000

pub const CONFIG_CH: LoRaChannel = LoRaChannel {
    freq: FREQUENCY as u32, // frequency in hertz
    bw: Bandwidth::Bw125kHz,
    sf: SpreadingFactor::Sf12,
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
    power: 10,
};

//let CONFIG_RADIO = Config::default() ;

pub const CONFIG_RADIO: radio_sx127x::device::Config = radio_sx127x::device::Config {
    modem: Modem::LoRa(CONFIG_LORA),
    channel: Channel::LoRa(CONFIG_CH),
    pa_config: CONFIG_PA,
    xtal_freq: 32000000, // CHECK
    timeout_ms: 100,
};

enum State<T>
where
    T: core::fmt::Debug + 'static,
{
    Reset,
    Idle,
    Sending,
    SendingDone,
    Received,
    Error(sx127xError<HalError<T, Infallible, Infallible>>),
}

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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut led = pins.led.into_push_pull_output();
    //blink(&mut led, &"Hello!");
    let _mosi_display = pins.gpio19.into_mode::<FunctionSpi>();
    let _sck_display = pins.gpio18.into_mode::<FunctionSpi>();
    let cs_display = pins.gpio17.into_push_pull_output();
    let dc_display = pins.gpio16.into_push_pull_output();

    let spi_display = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        65.MHz(),
        &embedded_hal_compat::eh0_2::spi::MODE_3,
    );
    // create a DisplayInterface from SPI and DC pin, with no manual CS control
    let di = SPIInterface::new(spi_display, dc_display, cs_display);
    // create the ILI9486 display driver in rgb666 color mode from the display interface and RST pin
    let mut display = Display::st7789(di, NoPin::default());
    display.init(&mut delay, DisplayOptions::default()).unwrap();
    // clear the display to black
    display.clear(Rgb565::BLUE).unwrap();
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    //let mut led_pin = pins.led.into_push_pull_output();

    let _miso = pins.gpio8.into_mode::<FunctionSpi>();
    let _mosi = pins.gpio11.into_mode::<FunctionSpi>();
    let _clk = pins.gpio10.into_mode::<FunctionSpi>();

    let spi = Spi::<_, _, 8>::new(pac.SPI1);
    let spi = spi
        .init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            20.MHz(),
            &embedded_hal_compat::eh0_2::spi::MODE_0,
        )
        .forward();

    let cs = pins.gpio9.into_readable_output().forward();
    let reset = pins.gpio7.into_readable_output().forward();
    let busy = pins.gpio12.into_floating_input().forward();
    let ready = pins.gpio13.into_floating_input().forward();

    // let mut btn = Button2::new(pins.gpio12.into_pull_up_input());

    let mut lora =
        Sx127x::spi(spi, cs, busy, ready, reset, delay.forward(), &CONFIG_RADIO).unwrap();

    //  let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay)
    //    .expect("Failed to communicate with radio module!");

    //lora.set_tx_power(17, 1); //Using PA_BOOST. See your board for correct pin.

    Text::new("Hello Rust!", Point::new(60, 60), style)
        .draw(&mut display)
        .unwrap();
    let mut cursor = 70;

    let message = "Bonjour la radio!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }
    let mut button = Button2::new(pins.gpio15.into_pull_up_input());
    let mut state = State::Idle;
    lora.start_receive().unwrap();

    loop {
        state = match state {
            State::Reset => State::Reset,
            State::Idle => {
                if button.just_pressed() {
                    lora.start_transmit(b"Kikooo");
                    State::Sending
                } else {
                    match lora.check_receive(false) {
                        Ok(true) => State::Received, //have a valid packet in the buffer
                        Ok(false) => State::Idle,    //got an invalid packet
                        Err(e) => State::Error(e),
                    }
                }
            }
            State::Sending => match lora.check_transmit() {
                Ok(true) => State::SendingDone,
                Ok(false) => State::Sending,
                Err(e) => State::Error(e),
            },
            State::Received => match (|| {
                let mut buff = [0u8; 256];
                let (len, info) = lora.get_received(&mut buff)?;
                info!(
                    "received packet len = {} info : {} {}",
                    len, info.rssi, info.snr
                );
                let mut str_buff = [0u8; 20];
                let text = len.numtoa_str(10, &mut str_buff);
                Text::new(text, Point::new(60, cursor), style)
                    .draw(&mut display)
                    .unwrap();
                let text = info.rssi.numtoa_str(10, &mut str_buff);
                Text::new(text, Point::new(60 + 6 * 5, cursor), style)
                    .draw(&mut display)
                    .unwrap();
                if let Some(snr) = info.snr {
                    let text = (snr).numtoa_str(10, &mut str_buff);
                    Text::new(text, Point::new(60 + 6 * 5 + 6 * 5, cursor), style)
                        .draw(&mut display)
                        .unwrap();
                }
                cursor = cursor + 10;
                Text::new(
                    unsafe { core::str::from_utf8_unchecked(&buff[..len]) },
                    Point::new(60, cursor),
                    style,
                )
                .draw(&mut display)
                .unwrap();
                cursor += 10;
                info!("got {},{},{}:{}", len, info.rssi, info.snr, buff[..len]);

                Ok(())
            })() {
                Ok(()) => State::Idle,
                Err(e) => State::Error(e),
            },
            State::SendingDone => match lora.start_receive() {
                Ok(()) => {
                    info!("Packet transmitted");
                    State::Idle
                }
                Err(e) => State::Error(e),
            },
            State::Error(e) => {
                debug!("{:?}", defmt::Debug2Format(&e));
                State::Reset
            }
        };
    }
}
