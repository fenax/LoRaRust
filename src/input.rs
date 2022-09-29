use core::{default, mem::size_of};

use bitmask_enum::bitmask;
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

// layout

// 00 01 02 03 14 15 17 28 29 30 31
// 07 06 05 04 13 16 18 27 26 25 24
//  08 09 10 11 12 19 20 21 22 13

//  q  w  e  r  t  y  u  i  o  p  *
//  ^  a  s  d  f  g  h  j  k  l  ^^
//   $  z  x  c  v  b  n  m  _  #

#[bitmask(u32)]
pub enum Keys {
    Q,
    W,
    E,
    R,
    D,
    S,
    A,
    ShiftL,
    Dollar,
    Z,
    X,
    C,
    V,
    F,
    T,
    Y,
    G,
    U,
    H,
    B,
    N,
    M,
    Underscore,
    Sharp,
    ShiftR,
    L,
    K,
    J,
    I,
    O,
    P,
    Star,
}

impl Keys {
    pub const Shift: Keys = Keys::ShiftR.or(Keys::ShiftL);
    pub const Modifiers: Keys = Keys::Star.or(Keys::Shift).or(Keys::Dollar).or(Keys::Sharp);

    pub fn get_one_char(self) -> Option<char> {
        let no_mod = self.and(Keys::Modifiers.not()).bits();
        let source = if self.intersects(Keys::Shift) {
            KEYS_CAPS
        } else if self.intersects(Keys::Dollar) {
            KEYS_NUM
        } else {
            KEYS_ALPHA
        };
        if no_mod.count_ones() == 1 {
            Some(source[no_mod.trailing_zeros() as usize])
        } else {
            None
        }
    }
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

pub fn is_key_pressed(data: Keys, key: Keys) -> bool {
    data.contains(key)
}

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

pub struct Keyboard<T>
where
    T: ReadRegister<u32>,
{
    reg: T,
}

impl<T> Keyboard<T>
where
    T: ReadRegister<u32>,
{
    pub fn new(reg: T) -> Self {
        Keyboard { reg }
    }
    pub fn get_keys(&mut self) -> Keys {
        self.reg.read().into()
    }
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
