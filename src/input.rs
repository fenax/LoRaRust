use core::{default, mem::size_of};

use embedded_hal_02::digital::v2::{InputPin, OutputPin};
use radio_sx127x::base::Hal;

pub struct Button<P>
where
    P: InputPin,
{
    button: P,
}

#[derive(Default)]
pub struct Modifier {
    pub star: bool,
    pub shift_l: bool,
    pub shift_r: bool,
    pub dollar: bool,
    pub sharp: bool,
}

pub fn extract_modifiers(data: u32) -> (Modifier, u32) {
    let mut modifier = Modifier::default();

    if (data & 0x80000000) != 0 {
        modifier.star = true;
    }
    if (data & 0x00000080) != 0 {
        modifier.shift_l = true;
    }
    if (data & 0x01000000) != 0 {
        modifier.shift_r = true;
    }
    if (data & 0x00000100) != 0 {
        modifier.dollar = true;
    }
    if (data & 0x00800000) != 0 {
        modifier.sharp = true;
    }
    (modifier, data & 0x7E7FFE7F)
}

pub const KEYS_ALPHA: [char; 32] = [
    'q', 'w', 'e', 'r', 'd', 's', 'a', '^', '$', 'z', 'x', 'c', 'v', 'f', 't', 'y', 'g', 'u', 'h',
    'b', 'n', 'm', '_', '#', '%', 'l', 'k', 'j', 'i', 'o', 'p', '*',
];
pub const KEYS_CAPS: [char; 32] = [
    'Q', 'W', 'E', 'R', 'D', 'S', 'A', '^', '$', 'Z', 'X', 'C', 'V', 'F', 'T', 'Y', 'G', 'U', 'H',
    'B', 'N', 'M', '_', '#', '%', 'L', 'K', 'J', 'I', 'O', 'P', '*',
];
pub const KEYS_NUM: [char; 32] = [
    '1', '2', '3', '4', 'd', 's', 'a', '^', '$', 'z', 'x', 'c', 'v', 'f', '5', '6', 'g', '7', 'h',
    'b', 'n', 'm', '_', '#', '%', 'l', 'k', 'j', '8', '9', '0', '*',
];

pub fn get_one_char_from(data: u32, source: &[char; 32]) -> Option<char> {
    if data.count_ones() == 1 {
        Some(source[data.trailing_zeros() as usize])
    } else {
        None
    }
}

pub fn get_one_char(data: u32) -> Option<char> {
    get_one_char_from(data, &KEYS_ALPHA)
}

pub struct ShiftRegister<CLK, DATA, LATCH, VAL>
where
    CLK: OutputPin,
    DATA: InputPin,
    LATCH: OutputPin,
{
    clk: CLK,
    data: DATA,
    latch: LATCH,
    phantom_data: core::marker::PhantomData<VAL>,
}

pub trait ReadRegister<VAL> {
    fn read(&mut self) -> VAL;
}
impl<CLK, DATA, LATCH, VAL> ShiftRegister<CLK, DATA, LATCH, VAL>
where
    CLK: OutputPin,
    DATA: InputPin,
    LATCH: OutputPin,
    DATA::Error: core::fmt::Debug,
    VAL: core::ops::ShlAssign + Default + core::ops::AddAssign + From<bool> + Copy,
{
    pub fn new(clk: CLK, data: DATA, latch: LATCH) -> Self {
        Self {
            clk,
            data,
            latch,
            phantom_data: core::marker::PhantomData::default(),
        }
    }
}
impl<CLK, DATA, LATCH, VAL> ReadRegister<VAL> for ShiftRegister<CLK, DATA, LATCH, VAL>
where
    CLK: OutputPin,
    DATA: InputPin,
    LATCH: OutputPin,
    DATA::Error: core::fmt::Debug,
    VAL: core::ops::ShlAssign + Default + core::ops::AddAssign + From<bool> + Copy,
{
    fn read(&mut self) -> VAL {
        let mut v = VAL::default();
        let one: VAL = true.into();

        let bits = size_of::<VAL>() * 8;

        self.clk.set_low();
        self.latch.set_low();
        cortex_m::asm::delay(10);
        self.latch.set_high();

        for _ in 0..bits {
            v <<= one;
            cortex_m::asm::delay(10);

            if self.data.is_high().unwrap() {
                v += one;
            }
            self.clk.set_high();
            cortex_m::asm::delay(10);

            self.clk.set_low();
        }

        v
    }
}

impl<P> Button<P>
where
    P: InputPin,
{
    pub fn new(pin: P) -> Self {
        Self { button: pin }
    }
    pub fn wait(&self) -> Result<(), P::Error> {
        //todo add debounce
        while self.button.is_high()? {}
        while self.button.is_low()? {}
        Ok(())
    }
}

pub struct Button2<P>
where
    P: InputPin,
{
    button: P,
    state: bool,
}

impl<P> Button2<P>
where
    P: InputPin,
    P::Error: core::fmt::Debug,
{
    pub fn new(pin: P) -> Self {
        Self {
            button: pin,
            state: false,
        }
    }
    pub fn just_pressed(&mut self) -> bool {
        if self.button.is_low().unwrap() {
            if self.state {
                false
            } else {
                self.state = true;
                true
            }
        } else {
            self.state = false;
            false
        }
    }
}
