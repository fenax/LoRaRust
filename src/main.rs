//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

mod blink;
mod stuff;
use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal_compat::eh0_2::digital::v2::OutputPin;
use embedded_keypad::keypad::*;
use panic_probe as _;
use shift_register::{
    input::{ReadRegister, ShiftRegister},
    *,
};

use stuff::{Delay10Mhz, Keys};

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
/*
pub struct Keyboard<T, ERR>
where
    ERR: core::fmt::Debug,
    T: input::ReadRegister<u32>,
{
    reg: T,
    phantom: core::marker::PhantomData<ERR>,
}

impl<T, ERR> Keyboard<T, ERR>
where
    ERR: core::fmt::Debug,
    T: input::ReadRegister<u32>,
{
    pub fn new(reg: T) -> Self {
        Keyboard {
            reg,
            phantom: Default::default(),
        }
    }
    pub fn get_keys(&mut self) -> u32 {
        self.reg.read()
    }
}
*/

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
    let mut pull_up = pins.gpio17.into_push_pull_output();
    let k_clk = pins.gpio15.into_push_pull_output();
    let k_data = pins.gpio16.into_floating_input();
    let k_latch = pins.gpio14.into_push_pull_output();
    _ = pull_up.set_high();
    let mut buffer = InputBuffer::<128, Keys>::new();
    buffer.left = Keys::Q | Keys::Star;
    buffer.right = Keys::E | Keys::Star;
    buffer.backspace = Keys::Star | Keys::ShiftR;
    buffer.validate = Keys::Return;

    let mut keyboard: ShiftRegister<_, _, _, u32, Delay10Mhz> =
        input::ShiftRegister::new(k_clk, k_data, k_latch);
    //blink::blink(&mut led_pin, "squee");
    loop {
        let key = keyboard.read();
        match buffer.process_input(key.into()) {
            InputState::Running(key) => {
                let key = key.and(Keys::Modifiers);
                /*if key == Keys::Dollar {
                    interface.set_overlay(Some(input::LAYOUT_NUM));
                } else {
                    interface.set_overlay(None);
                }*/
            }
            InputState::Updated => {
                //interface.set_input(buffer.get_data(), buffer.get_cursor());
                info!("{}", buffer);
            }
            InputState::Overflow => {
                info!("Overflow");
            }
            InputState::Validated => {
                info!("SENDING {}", buffer);
                //interface.set_input(b"", 0);
                //sending = true;
            }
            InputState::NotForMe(_key) => {}
        }
    }
}

// End of file
