#![no_std]
#![no_main]
mod blink;
mod input;
mod interface;
mod stuff;

use embedded_graphics::text::renderer::TextRenderer;
use stuff::*;

use bsp::{entry, hal::gpio::FunctionSpi};

use defmt::*;
use defmt_rtt as _;
use embedded_hal_compat::eh0_2::digital::v2::InputPin;
use embedded_hal_compat::eh0_2::digital::v2::OutputPin;
//use embedded_hal_compat::eh0_2::spi::{Mode, Phase, Polarity, MODE_0};
//use embedded_hal_compat::eh1_0::spi::blocking::{Transactional, TransferInplace, Write};
use embedded_hal_compat::ForwardCompat;
use fugit::RateExtU32;
use input::*;
use numtoa::NumToA;
use panic_probe as _;
use ssd1681::prelude::*;

use display_interface_spi::SPIInterface;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::PrimitiveStyleBuilder,
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
    let _miso_display = pins.gpio4.into_mode::<FunctionSpi>();
    let _mosi_display = pins.gpio3.into_mode::<FunctionSpi>();
    let _sck_display = pins.gpio2.into_mode::<FunctionSpi>();
    let cs_display = pins.gpio5.into_push_pull_output();
    let dc_display = pins.gpio28.into_push_pull_output();
    let rst_display = pins.gpio0.into_push_pull_output();
    let busy_display = pins.gpio1.into_floating_input();

    let mut spi_display = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        20.MHz(),
        &embedded_hal_compat::eh0_2::spi::MODE_3,
    );
    // create a DisplayInterface from SPI and DC pin, with no manual CS control
    //let di = SPIInterface::new(spi_display, dc_display, cs_display);
    // create the ILI9486 display driver in rgb666 color mode from the display interface and RST pin

    let mut eink = Ssd1681::new(
        &mut spi_display,
        cs_display,
        busy_display,
        dc_display,
        rst_display,
        &mut delay,
    )
    .unwrap();
    eink.clear_red_frame(&mut spi_display).unwrap();
    eink.clear_bw_frame(&mut spi_display).unwrap();

    let mut display_bw = Display1in54::bw();
    let mut display_red = Display1in54::red();

    //let mut display = Display::st7789(di, NoPin::default());
    //display.init(&mut delay, DisplayOptions::default()).unwrap();
    // clear the display to black
    eink.update_red_frame(&mut spi_display, display_red.buffer())
        .unwrap();

    eink.update_bw_frame(&mut spi_display, display_bw.buffer())
        .unwrap();
    eink.display_frame(&mut spi_display).unwrap();
    //display.clear(Rgb565::BLUE).unwrap();
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let clear_style = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::Off)
        .build();

    let _miso = pins.gpio8.into_mode::<FunctionSpi>();
    let _mosi = pins.gpio11.into_mode::<FunctionSpi>();
    let _clk = pins.gpio10.into_mode::<FunctionSpi>();

    let cs = pins.gpio9.into_readable_output().forward();
    let reset = pins.gpio7.into_readable_output().forward();
    let busy = pins.gpio12.into_floating_input().forward();
    let ready = pins.gpio13.into_floating_input().forward();

    let spi = Spi::<_, _, 8>::new(pac.SPI1);
    let spi = spi
        .init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            20.MHz(),
            &embedded_hal_compat::eh0_2::spi::MODE_0,
        )
        .forward();

    delay.delay_ms(1000);
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
        Point::new(0, 6),
        style,
    )
    .draw(&mut display_bw)
    .unwrap();
    eink.update_red_frame(&mut spi_display, display_red.buffer())
        .unwrap();

    eink.update_bw_frame(&mut spi_display, display_bw.buffer())
        .unwrap();
    eink.display_frame(&mut spi_display).unwrap();
    //display.flush().unwrap();
    //crate::panic!("aaaaa");

    let mut lora = res.unwrap();

    //lora.set_tx_power(17, 1); //Using PA_BOOST. See your board for correct pin.

    let mut pull_up = pins.gpio17.into_push_pull_output();
    let k_clk = pins.gpio15.into_push_pull_output();
    let k_data = pins.gpio16.into_floating_input();
    let k_latch = pins.gpio14.into_push_pull_output();
    _ = pull_up.set_high();

    let mut keyboard = Keyboard::new(ShiftRegister::new(k_clk, k_data, k_latch));
    let mut state = State::Init;
    let mut buffer = InputBuffer::<128>::new();
    //let mut str: String<128> = String::new();

    let cursor = 6;
    let mut sending = false;
    // TODO :  drawing above line 6 causes garbage
    //Text::new("Otterly radiolifique", Point::new(0, 6), style)
    //    .draw(&mut display)
    //    .unwrap();
    //display.flush().unwrap();
    /*let mut disp = Disp {
        display,
        cursor,
        style,
    };*/
    let mut interface = Oled128x128::new();
    interface.set_title(b"Rusty Communicator");

    /*
    interface.set_input(b"input", 0);
    interface.set_title(b"title");
    interface.add_log(b"Squee squee", Some(19), Some(-5));
    interface.add_log(b"PAtapatate", Some(123), Some(555));
    interface.add_log(b"Voyage Voyage", None, None);
    interface.add_log(b"Loutre", None, None);
    interface.add_log(b"Avion", None, None);
    interface.add_log(b"Renard", None, None);
    interface.add_log(b"Carotte", None, None);
    interface.add_log(b"Chips", None, None);
    interface.add_log(b"Blop", None, None);
    */

    interface.draw(&mut display_bw);

    //display.flush();
    eink.update_red_frame(&mut spi_display, display_red.buffer())
        .unwrap();

    eink.update_bw_frame(&mut spi_display, display_bw.buffer())
        .unwrap();
    eink.display_frame(&mut spi_display).unwrap();

    loop {
        /*Rectangle::new(Point::new(0, 118), Size::new(128, 10))
            .into_styled(clear_style)
            .draw(&mut disp.display)
            .unwrap();
        Text::new(
            unsafe { core::str::from_utf8_unchecked(buffer.get_data()) },
            Point::new(0, 124),
            disp.style,
        )
        .draw(&mut disp.display)
        .unwrap();*/
        if !sending {
            let key = keyboard.get_keys();
            match buffer.process_input(key) {
                InputState::Running(key) => {
                    let key = key.and(Keys::Modifiers);
                    if key == Keys::Dollar {
                        interface.set_overlay(Some(input::LAYOUT_NUM));
                    } else {
                        interface.set_overlay(None);
                    }
                }
                InputState::Updated => {
                    interface.set_input(buffer.get_data(), buffer.get_cursor());
                    info!("{}", buffer);
                }
                InputState::Overflow => {
                    info!("Overflow");
                }
                InputState::Validated => {
                    info!("SENDING {}", buffer);
                    interface.set_input(b"", 0);
                    sending = true;
                }
                InputState::NotForMe(_key) => {}
            }
        }
        state = match state.run_state(&mut lora, &mut sending, &mut buffer, &mut interface) {
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
        };
        //Pixel(Point::new(127, 127), BinaryColor::On).draw(&mut disp.display);
        interface.draw(&mut display_bw);
        //display.flush().unwrap();
        eink.update_red_frame(&mut spi_display, display_red.buffer())
            .unwrap();

        eink.update_bw_frame(&mut spi_display, display_bw.buffer())
            .unwrap();
        eink.display_frame(&mut spi_display).unwrap();
    }
}

use core::fmt::Debug;

use crate::interface::{Interface, Oled128x128};

impl State {
    fn run_state<Hal: radio_sx127x::base::Hal, T: Debug + 'static /* , D, S*/>(
        &self,
        lora: &mut radio_sx127x::Sx127x<Hal>,
        sending: &mut bool,
        send_buffer: &mut InputBuffer<128>,
        disp: &mut impl Interface,
        //disp: &mut Disp<D, S>,
    ) -> Result<Self, stuff::Error<T>>
    where
        stuff::Error<T>: From<sx127xError<T>>,
        //D::Error: Debug,
        stuff::Error<T>: From<radio_sx127x::Error<<Hal as radio_sx127x::base::Hal>::Error>>,
        //        DI: display_interface::WriteOnlyDataCommand,
        //        RST: OutputPin,
        //        MODEL: mipidsi::models::Model,
        //D: DrawTarget<Color = <S as TextRenderer>::Color>,
        //S: embedded_graphics::text::renderer::TextRenderer + Copy,
        //D: DrawTarget<Color = <S as TextRenderer>::Color>,
        //S: embedded_graphics::text::renderer::TextRenderer + Copy,
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
                if *sending {
                    info!("Send packet");
                    lora.start_transmit(send_buffer.get_data())?;
                    send_buffer.clear();
                    *sending = false;
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
                    "received packet len = {} info : {} {}{}",
                    len, info.rssi, info.snr, buff
                );
                disp.add_log(&buff[..len], info.snr, Some(info.rssi));
                //Ok(Self::Idle)
                //lora.start_transmit(&buff[..len])?;
                Ok(State::PrepareIdle)
            }
            State::SendingDone => {
                lora.start_receive()?;
                Ok(Self::Idle)
            }
        }
    }
}
