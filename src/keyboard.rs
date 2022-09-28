#![no_std]
#![no_main]
mod blink;
mod input;
mod stuff;

use bsp::{entry, hal::gpio::FunctionSpi};
use embedded_graphics::text::renderer::TextRenderer;
use heapless::String;
use input::*;
use stuff::*;

use defmt::*;
use defmt_rtt as _;
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
    //    let cs_display = pins.gpio17.into_push_pull_output();
    //    let dc_display = pins.gpio16.into_push_pull_output();

    let spi_display = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        65.MHz(),
        &embedded_hal_compat::eh0_2::spi::MODE_3,
    );
    // create a DisplayInterface from SPI and DC pin, with no manual CS control
    //let di = SPIInterface::new(spi_display, dc_display, cs_display);
    // create the ILI9486 display driver in rgb666 color mode from the display interface and RST pin
    //let mut display = Display::st7789(di, NoPin::default());
    //display.init(&mut delay, DisplayOptions::default()).unwrap();
    // clear the display to black
    //display.clear(Rgb565::BLUE).unwrap();
    //let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    //let mut led_pin = pins.led.into_push_pull_output();

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

    let mut lora =
        Sx127x::spi(spi, cs, busy, ready, reset, delay.forward(), &CONFIG_RADIO).unwrap();

    //lora.set_tx_power(17, 1); //Using PA_BOOST. See your board for correct pin.

    let mut pull_up = pins.gpio17.into_push_pull_output();
    let k_clk = pins.gpio15.into_push_pull_output();
    let k_data = pins.gpio16.into_floating_input();
    let k_latch = pins.gpio14.into_push_pull_output();
    pull_up.set_high();

    let mut keyboard: ShiftRegister<_, _, _, u32> = ShiftRegister::new(k_clk, k_data, k_latch);
    let mut state = State::Init;
    let mut str: String<128> = String::new();
    let mut last = 0u32;
    let mut ready = true;
    let mut sending = false;
    loop {
        if sending == false {
            let key = keyboard.read();
            if key != last {
                let (m, c) = extract_modifiers(key);
                let default = get_one_char(c);

                let modified = get_one_char_from(
                    c,
                    if m.shift_l || m.shift_r {
                        &KEYS_CAPS
                    } else if m.dollar {
                        &KEYS_NUM
                    } else {
                        &KEYS_ALPHA
                    },
                );
                if modified.is_none() {
                    ready = true;
                }
                if let (Some(car), true) = (modified, ready) {
                    _ = str.push(car);
                    info!("{}", str.as_str());
                    ready = false;
                }

                if ready && m.sharp {
                    info!("SENDING {}", str.as_str());
                    sending = true;
                }
                /*info!(
                    "key is {:08X},{}{}{}{}{} {}    {}",
                    key,
                    if m.star { '*' } else { ' ' },
                    if m.shift_l { '^' } else { ' ' },
                    if m.shift_r { '%' } else { ' ' },
                    if m.dollar { '$' } else { ' ' },
                    if m.sharp { '#' } else { ' ' },
                    default,
                    modified
                )*/
            }
            last = key;
        }
        state = match state.run_state(&mut lora, &mut sending, &mut str) {
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
use core::ptr::read;

impl State {
    fn run_state<Hal: radio_sx127x::base::Hal, T: Debug + 'static /* , D, S*/>(
        &self,
        lora: &mut radio_sx127x::Sx127x<Hal>,
        sending: &mut bool,
        send_buffer: &mut String<128>,
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
                    lora.start_transmit(send_buffer.as_str().as_bytes())?;
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
                //Ok(Self::Idle)
                //lora.start_transmit(&buff[..len])?;
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
