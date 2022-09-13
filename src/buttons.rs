#![no_std]
#![no_main]
mod input;
use bsp::{entry, hal::gpio::FunctionSpi};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
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

use crate::input::{Button, Button2};

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

    let mut led_pin = pins.led.into_push_pull_output();

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

    let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay)
        .expect("Failed to communicate with radio module!");

    lora.set_tx_power(17, 1); //Using PA_BOOST. See your board for correct pin.

    let message = "Hello, world!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }
    let mut buffer2 = [0; 255];

    let mut button = Button2::new(pins.gpio19.into_pull_up_input());
    loop {
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
                }
            }
        }
    }
    /*
    loop {
        match button.wait() {
            Ok(_) => {
                let transmit = lora.transmit_payload_busy(buffer, message.len());
                match transmit {
                    Ok(packet_size) => info!("Sent packet with size: {}", packet_size),
                    Err(_) => info!("Error"),
                }
            }
            Err(_) => info!("erroroed"),
        }
    }*/
}
