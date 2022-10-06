#![no_std]
#![no_main]
mod blink;
mod input;
mod stuff;

use embedded_graphics::text::renderer::TextRenderer;
use stuff::*;

use bsp::{entry, hal::gpio::FunctionSpi};

use defmt::*;
use defmt_rtt as _;
use embedded_hal_compat::eh0_2::digital::v2::InputPin;
//use embedded_hal_compat::eh0_2::spi::{Mode, Phase, Polarity, MODE_0};
//use embedded_hal_compat::eh1_0::spi::blocking::{Transactional, TransferInplace, Write};
use embedded_hal_compat::ForwardCompat;
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

use radio_sx127x::prelude::*;
use radio_sx127x::Error as sx127xError; // Error name conflict with hals

use radio::{Receive, Transmit};

use crate::input::Button2;

struct Disp<D, S>
where
    //    DI: display_interface::WriteOnlyDataCommand,
    //    RST: OutputPin,
    //    MODEL: mipidsi::models::Model,
    D: DrawTarget,
    S: embedded_graphics::text::renderer::TextRenderer,
{
    display: D, //Display<DI, RST, MODEL>,
    cursor: i32,
    style: S,
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
    let mut _led = pins.led.into_push_pull_output();
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

    let res = Sx127x::spi(spi, cs, busy, ready, reset, delay.forward(), &CONFIG_RADIO);

    //  let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, delay)
    //    .expect("Failed to communicate with radio module!");

    //lora.set_tx_power(17, 1); //Using PA_BOOST. See your board for correct pin.

    Text::new(
        if res.is_ok() {
            "Hello Rust!"
        } else {
            "nooooooo"
        },
        Point::new(60, 60),
        style,
    )
    .draw(&mut display)
    .unwrap();
    let mut lora = res.unwrap();
    let cursor = 70;

    let message = "Bonjour la radio!";
    let mut buffer = [0; 255];
    for (i, c) in message.chars().enumerate() {
        buffer[i] = c as u8;
    }
    let mut button = Button2::new(pins.gpio15.into_pull_up_input());
    let mut state = State::Init;
    let mut disp = Disp {
        display,
        cursor,
        style,
    };

    loop {
        state = match state.run_state(&mut lora, &mut button, &mut disp) {
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

impl State {
    fn run_state<Hal: radio_sx127x::base::Hal, P: InputPin, T: Debug + 'static, D, S>(
        &self,
        lora: &mut radio_sx127x::Sx127x<Hal>,
        button: &mut Button2<P>,
        disp: &mut Disp<D, S>,
    ) -> Result<Self, stuff::Error<T>>
    where
        stuff::Error<T>: From<sx127xError<T>>,
        P::Error: Debug,
        D::Error: Debug,
        stuff::Error<T>: From<radio_sx127x::Error<<Hal as radio_sx127x::base::Hal>::Error>>,
        //        DI: display_interface::WriteOnlyDataCommand,
        //        RST: OutputPin,
        //        MODEL: mipidsi::models::Model,
        D: DrawTarget<Color = <S as TextRenderer>::Color>,
        S: embedded_graphics::text::renderer::TextRenderer + Copy,
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
                info!("to idle");
                lora.start_receive()?;

                Ok(State::Idle)
            }
            State::Idle => {
                if button.just_pressed() {
                    info!("Send packet");
                    lora.start_transmit(b"Kikooo")?;
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
                let mut str_buff = [0u8; 20];
                let text = len.numtoa_str(10, &mut str_buff);
                Text::new(text, Point::new(60, disp.cursor), disp.style)
                    .draw(&mut disp.display)
                    .unwrap();
                let text = info.rssi.numtoa_str(10, &mut str_buff);
                Text::new(text, Point::new(60 + 6 * 5, disp.cursor), disp.style)
                    .draw(&mut disp.display)
                    .unwrap();
                if let Some(snr) = info.snr {
                    let text = (snr).numtoa_str(10, &mut str_buff);
                    Text::new(
                        text,
                        Point::new(60 + 6 * 5 + 6 * 5, disp.cursor),
                        disp.style,
                    )
                    .draw(&mut disp.display)
                    .unwrap();
                }
                disp.cursor = disp.cursor + 10;
                Text::new(
                    unsafe { core::str::from_utf8_unchecked(&buff[..len]) },
                    Point::new(60, disp.cursor),
                    disp.style,
                )
                .draw(&mut disp.display)
                .unwrap();
                disp.cursor += 10;
                info!("got {},{},{}:{}", len, info.rssi, info.snr, buff[..len]);

                Ok(State::PrepareIdle)
            }
            State::SendingDone => {
                lora.start_receive()?;
                Ok(Self::Idle)
            }
            State::Panic => {
                crate::panic!("panic")
            }
        }
    }
}
