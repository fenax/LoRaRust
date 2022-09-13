#![no_std]
#![no_main]
mod blink;
mod input;
use blink::blink;
use bsp::{
    entry,
    hal::gpio::{FunctionSpi, FunctionUsbAux},
};
use defmt::*;
use defmt_rtt as _;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::*,
};
use embedded_hal::digital::v2::OutputPin;
use fugit::RateExtU32;
use input::*;
use mipidsi::*;
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
    let _mosi_display = pins.gpio19.into_mode::<FunctionSpi>();
    let _sck_display = pins.gpio18.into_mode::<FunctionSpi>();
    let cs_display = pins.gpio17.into_push_pull_output();
    let dc_display = pins.gpio16.into_push_pull_output();

    let spi_display = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        65.MHz(),
        &embedded_hal::spi::MODE_3,
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
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        20.MHz(),
        &embedded_hal::spi::MODE_0,
    );

    let cs = pins.gpio9.into_push_pull_output();
    let reset = pins.gpio7.into_push_pull_output();

    let mut btn = Button2::new(pins.gpio12.into_pull_up_input());

    let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay)
        .expect("Failed to communicate with radio module!");

    lora.set_tx_power(17, 1); //Using PA_BOOST. See your board for correct pin.

    Text::new("Hello Rust!", Point::new(60, 60), style)
        .draw(&mut display)
        .unwrap();
    let mut cursor = 70;

    let message = "Bonjour la radio!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }

    loop {
        match lora.poll_irq(Some(100)) {
            Ok(size) => {
                let mut str_buff = [0u8; 20];
                let text = size.numtoa_str(10, &mut str_buff);
                Text::new(text, Point::new(60, cursor), style)
                    .draw(&mut display)
                    .unwrap();
                let rssi = lora.get_packet_rssi();
                let snr = lora.get_packet_snr();
                if let Ok(rssi) = rssi {
                    let text = rssi.numtoa_str(10, &mut str_buff);
                    Text::new(text, Point::new(60 + 6 * 4, cursor), style)
                        .draw(&mut display)
                        .unwrap();
                }
                if let Ok(snr) = snr {
                    let text = (snr as i32).numtoa_str(10, &mut str_buff);
                    Text::new(text, Point::new(60 + 6 * 4 + 6 * 4, cursor), style)
                        .draw(&mut display)
                        .unwrap();
                }
                cursor += 10;
                match lora.read_packet() {
                    Ok(result) => {
                        Text::new(
                            unsafe { core::str::from_utf8_unchecked(&result[..size]) },
                            Point::new(60, cursor),
                            style,
                        )
                        .draw(&mut display)
                        .unwrap();
                        cursor += 10;
                        info!("got {},{},{}:{}", size, rssi.unwrap(), snr.unwrap(), result);
                    }
                    Err(_) => info!("fail packet"),
                }
            }
            Err(_) =>
            //timeout
            {
                if btn.just_pressed() {
                    let transmit = lora.transmit_payload_busy(buffer, message.len());
                    match transmit {
                        Ok(packet_size) => info!("Sent packet with size: {}", packet_size),
                        Err(_) => info!("Error"),
                    }
                    lora.set_mode(sx127x_lora::RadioMode::RxContinuous);
                }
            }
        }
    }
}
