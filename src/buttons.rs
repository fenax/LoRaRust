#![no_std]
#![no_main]
mod blink;
mod input;

use blink::blink;
use bsp::hal::spi::Enabled;
use bsp::{entry, hal::gpio::FunctionSpi};
use defmt::export::panic;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::delay::blocking::DelayUs;
use embedded_hal::digital::blocking::{InputPin, OutputPin};
//use embedded_hal::spi::blocking::SpiDevice;
//{Transfer, Write};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::spi::{Transactional, Transfer, Write};
//use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal::spi::{Mode, Phase, Polarity, MODE_0};

use fugit::RateExtU32;
use numtoa::NumToA;
use panic_probe as _;

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

use radio_sx127x::Error as sx127xError; // Error name conflict with hals
use radio_sx127x::{
    device::lora::{
        Bandwidth, CodingRate, FrequencyHopping, LoRaChannel, LoRaConfig, PayloadCrc,
        PayloadLength, SpreadingFactor,
    },
    device::{Channel, Modem, PaConfig, PaSelect},
    prelude::*, // prelude has Sx127x,
};

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

enum State {
    Reset,
    Idle,
    Sending,
    SendingDone,
    Receiving,
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

    let _miso = pins.gpio8.into_mode::<FunctionSpi>();
    let _mosi = pins.gpio11.into_mode::<FunctionSpi>();
    let _clk = pins.gpio10.into_mode::<FunctionSpi>();

    let spi = Spi::<_, _, 8>::new(pac.SPI1);
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        20.MHz(),
        &MODE_0,
    );

    let cs = pins.gpio9.into_push_pull_output();
    let reset = pins.gpio7.into_push_pull_output();
    let busy = pins.gpio12.into_floating_input();
    let ready = pins.gpio13.into_floating_input();

    let lora = Sx127x::spi(spi, cs, busy, ready, reset, delay, &CONFIG_RADIO).unwrap();
    /*let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay).unwrap_or_else(|_x| {
        blink(&mut led, "module");
        crate::panic!("Failed to communicate with radio module!");
    });*/

    let message = "Hello, world!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }
    let mut cursor = 0;
    let mut button = Button2::new(pins.gpio19.into_pull_up_input());
    let mut state = State::Idle;
    loop {
        match state {
            State::Reset => {}
            State::Idle => {}
            State::Sending => {}
            State::Receiving => {}
            State::SendingDone => {}
        }
        /*
        match lora.poll_irq(Some(100)) {
            Ok(size) => {
                let mut cursor = 0;
                let mut str_buff = [0u8; 20];
                let text = size.numtoa(10, &mut str_buff);
                for c in text {
                    buffer2[cursor] = *c;
                    cursor += 1;
                }
                buffer2[cursor] = b',';
                cursor += 1;
                let rssi = lora.get_packet_rssi();
                let snr = lora.get_packet_snr();
                if let Ok(rssi) = rssi {
                    let text = rssi.numtoa(10, &mut str_buff);
                    for c in text {
                        buffer2[cursor] = *c;
                        cursor += 1;
                    }
                    buffer2[cursor] = b',';
                    cursor += 1;
                }
                if let Ok(snr) = snr {
                    let text = (snr as i32).numtoa(10, &mut str_buff);
                    for c in text {
                        buffer2[cursor] = *c;
                        cursor += 1;
                    }
                    buffer2[cursor] = b',';
                    cursor += 1;
                }
                match lora.read_packet() {
                    Ok(result) => {
                        for c in &result[..size] {
                            buffer2[cursor] = *c;
                            cursor += 1;
                        }
                        let transmit = lora.transmit_payload_busy(buffer2, message.len());
                        match transmit {
                            Ok(packet_size) => info!("Sent packet with size: {}", packet_size),
                            Err(_) => info!("Error"),
                        }
                        lora.set_mode(RadioMode::RxContinuous);
                        info!("got {},{},{}:{}", size, rssi.unwrap(), snr.unwrap(), result);
                    }
                    Err(_) => info!("fail packet"),
                }
            }
            Err(_) =>
            //timeout
            {
                if button.just_pressed() {
                    let transmit = lora.transmit_payload_busy(buffer, message.len());
                    match transmit {
                        Ok(packet_size) => info!("Sent packet with size: {}", packet_size),
                        Err(_) => info!("Error"),
                    }
                    lora.set_mode(RadioMode::RxContinuous);
                }
            }
        }*/
    }
}

fn copy(src: &[u8], target: &mut [u8], cursor: &mut usize) {
    for c in src {
        if *cursor >= target.len() {
            return;
        }
        target[*cursor] = *c;
        *cursor += 1;
    }
}
