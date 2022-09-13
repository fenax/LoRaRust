#![no_std]
#![no_main]
mod blink;
mod input;

use blink::blink;
use bsp::{entry, hal::gpio::FunctionSpi};
use defmt::export::panic;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::{InputPin, OutputPin};
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
use sx127x_lora::RadioMode;

use crate::input::Button2;

//use hal::spidev::{self, SpidevOptions};
//use hal::sysfs_gpio::Direction;
//use bsp::hal::Delay;
//use hal::{Pin, Spidev};

const FREQUENCY: i64 = 434;

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
    blink(&mut led, &"Hello!");

    let _miso = pins.gpio8.into_mode::<FunctionSpi>();
    let _mosi = pins.gpio11.into_mode::<FunctionSpi>();
    let _clk = pins.gpio10.into_mode::<FunctionSpi>();

    let spi = Spi::<_, _, 8>::new(pac.SPI1);
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        20.MHz(),
        &embedded_hal::spi::MODE_0,
    );

    let cs = pins.gpio9.into_push_pull_output();
    let reset = pins.gpio7.into_push_pull_output();

    let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay).unwrap_or_else(|_x| {
        blink(&mut led, "module");
        crate::panic!("Failed to communicate with radio module!");
    });

    lora.set_tx_power(17, 1).unwrap_or_else(|_| {
        //Using PA_BOOST. See your board for correct pin.
        blink(&mut led, "power");
        crate::panic!("Failed setting module power");
    });
    let message = "Hello, world!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }
    let mut cursor = 0;
    let mut button = Button2::new(pins.gpio19.into_pull_up_input());
    loop {
        if let Err(e) =
            application_loop(&mut lora, &mut button, &mut cursor, &buffer, message.len())
        {
            match e {
                Error::FailedTx => {}
                Error::FailedRx => {}
                Error::Hardware => blink(&mut led, "Hard Fail"),
                Error::Busy => {}
            }
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

enum Error {
    FailedTx,
    FailedRx,
    Hardware,
    Busy,
}

fn lora_rx<SPI, CS, RESET>(error: sx127x_lora::Error<SPI, CS, RESET>) -> Error {
    match error {
        sx127x_lora::Error::Uninformative => Error::Hardware,
        sx127x_lora::Error::VersionMismatch(_) => Error::Hardware,
        sx127x_lora::Error::CS(_) => Error::Hardware,
        sx127x_lora::Error::Reset(_) => Error::Hardware,
        sx127x_lora::Error::SPI(_) => Error::Hardware,
        sx127x_lora::Error::Transmitting => Error::Busy,
    }
}

fn lora_tx<SPI, CS, RESET>(error: sx127x_lora::Error<SPI, CS, RESET>) -> Error {
    match error {
        sx127x_lora::Error::Uninformative => Error::Hardware,
        sx127x_lora::Error::VersionMismatch(_) => Error::Hardware,
        sx127x_lora::Error::CS(_) => Error::Hardware,
        sx127x_lora::Error::Reset(_) => Error::Hardware,
        sx127x_lora::Error::SPI(_) => Error::Hardware,
        sx127x_lora::Error::Transmitting => Error::Busy,
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

fn application_loop<SPI, CS, RESET, DELAY, E, B>(
    lora: &mut sx127x_lora::LoRa<SPI, CS, RESET, DELAY>,
    button: &mut Button2<B>,
    cursor: &mut usize,
    message: &[u8; 255],
    len: usize,
) -> Result<(), Error>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E>,
    CS: OutputPin,
    RESET: OutputPin,
    DELAY: DelayMs<u8>,
    B: InputPin,
    B::Error: core::fmt::Debug,
{
    let mut buffer2 = [0; 255];

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
            let rssi = lora.get_packet_rssi().map_err(lora_rx)?;
            let snr = lora.get_packet_snr().map_err(lora_rx)?;
            let text = rssi.numtoa(10, &mut str_buff);
            copy(&text, &mut buffer2, &mut cursor);
            copy(&[b','], &mut buffer2, &mut cursor);

            let text = (snr as i32).numtoa(10, &mut str_buff);
            copy(&text, &mut buffer2, &mut cursor);
            copy(&[b','], &mut buffer2, &mut cursor);

            let result = lora.read_packet().map_err(lora_rx)?;
            copy(&result[..size], &mut buffer2, &mut cursor);

            let transmit = lora
                .transmit_payload_busy(buffer2, cursor)
                .map_err(lora_tx)?;
            info!("Sent packet with size: {}", transmit);

            lora.set_mode(RadioMode::RxContinuous).map_err(lora_rx)?;
            info!("got {},{},{}:{}", size, rssi, snr, result);
            Ok(())
        }
        Err(_) =>
        //timeout
        {
            if button.just_pressed() {
                let transmit = lora.transmit_payload_busy(*message, len).map_err(lora_tx)?;
                info!("Sent packet with size: {}", transmit);

                lora.set_mode(RadioMode::RxContinuous).map_err(lora_rx)?;
            }
            Ok(())
        }
    }
}
