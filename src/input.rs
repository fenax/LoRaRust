#![allow(dead_code)]

use core::mem::size_of;

use bitmask_enum::bitmask;
use defmt::intern;
use defmt::Format;
use embedded_hal_02::digital::v2::{InputPin, OutputPin};
use paste::paste;

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

macro_rules! count_tt {
    () => { 0 };
    ($odd:tt $($a:tt $b:tt)*) => { (count_tt!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_tt!($($a)*) << 1 };
}

macro_rules! build_keyboard {
    ($type_name : ident,$data_size:ty,
        [
            $($key_codes:literal)*
        ],
        [
            $($key_names:ident)*
        ],
        [$(
            [$modifier:ident [$($modifier_key:ident),*]]
        ),*]
    ) => {
        #[bitmask($data_size)]
        pub enum $type_name {
            $(
                    $key_names = 1<< $key_codes,
            )*
            $(
                $modifier = {
                    let mut val = 0;
                    val |= $(Self::$modifier_key.bits;)*
                    val
                },
            )*
        }
        impl $type_name{
            const KEYMAP_SIZE:usize=count_tt!($($key_codes)*);
            const KEYMAP:[usize;$type_name::KEYMAP_SIZE] = [$($key_codes),*];
        }
    };
}

build_keyboard!(Keys2,u32,
[00 01 02 03 14 15 17 28 29 30 31
 07 06 05 04 13 16 18 27 26 25 24
 08 09 10 11 12 19 20 21 22 23],
[Q W E R T Y U I O P Star
  ShiftL  A S D F G H J K L ShiftR
  Dollar  Z X C V B N M Space Return],
[[Modifiers [Star, ShiftL, ShiftR, Dollar, Return]],
  [Shift[ShiftR, ShiftL]],
  [TextMod[Shift, Dollar]]
  ]

);

macro_rules! build_keymap {
    (   $name:ident,
        $modifiers:ident,
        $textmod:ident,
        $default:literal,
        $format:literal,
        [
            $($($mod:ident)* [$map:literal]),*
        ]
    ) => {
        paste!{
            impl $name {
                const fn make_map(keys:&[u8;$name::KEYMAP_SIZE])->[u8;$name::KEYMAP_SIZE]{
                    let mut out = [0u8;$name::KEYMAP_SIZE];
                    let mut i = 0usize;
                    loop{
                        out[$name::KEYMAP[i]] = keys[i];
                        i +=1;
                        if i ==$name::KEYMAP_SIZE{
                            break
                        }
                    }
                    out
                }
                const DEFAULT_MAP : [u8;$name::KEYMAP_SIZE] = $name::make_map($default);
                $(
                    const [<$($mod:upper _)* MAP>] : [u8;$name::KEYMAP_SIZE] = $name::make_map($map);
                )*

                pub fn get_one_char(self) -> Option<u8> {
                    let no_mod = self.and($name::$modifiers.not()).bits();
                    let mods = self.and($name::$modifiers);
                    if mods.and($name::$textmod.not()).is_none(){
                        let source = $(
                            if $( mods.intersects($name::$mod) &&)* true{
                                $name::[<$($mod:upper _)* MAP>]
                            }else
                        )*
                        {
                            $name::DEFAULT_MAP
                        };
                        if no_mod.count_ones() == 1 {
                            Some(source[no_mod.trailing_zeros() as usize])
                        } else {
                            None
                        }
                    }else{
                        None
                    }
                }
            }
        }

    };
}

build_keymap!(
    Keys2,
    Modifiers,
    TextMod,
    b"qwertyuiop*^asdfghjkl^*zxcvbnm *",
    b"* * * * * * * * * * *\n* * * * * * * * * * *\n * * * * * * * * * *",
    [
        Shift[b"QWERTYUIOP**ASDFGHJKL**ZXCVBNM *"],
        Dollar[b"1234567890**&@\"#()=$%**,;.:!?'_*"]
    ]
);

// layout

pub const KEYS_ALPHA: [char; 32] = [
    'q', 'w', 'e', 'r', 'd', 's', 'a', '^', '$', 'z', 'x', 'c', 'v', 'f', 't', 'y', 'g', 'u', 'h',
    'b', 'n', 'm', ' ', '#', '%', 'l', 'k', 'j', 'i', 'o', 'p', '*',
];
pub const KEYS_CAPS: [char; 32] = [
    'Q', 'W', 'E', 'R', 'D', 'S', 'A', '^', '$', 'Z', 'X', 'C', 'V', 'F', 'T', 'Y', 'G', 'U', 'H',
    'B', 'N', 'M', ' ', '#', '%', 'L', 'K', 'J', 'I', 'O', 'P', '*',
];
pub const KEYS_NUM: [char; 32] = [
    '1', '2', '3', '4', '"', '@', '&', '^', '$', ',', ';', '.', ':', '#', '5', '6', '(', '7', ')',
    '!', '?', '\'', '_', '#', '%', '%', '$', '=', '8', '9', '0', '*',
];

// 00 01 02 03 14 15 17 28 29 30 31
// 07 06 05 04 13 16 18 27 26 25 24
//  08 09 10 11 12 19 20 21 22 23

//  q  w  e  r  t  y  u  i  o  p  *
//  ^  a  s  d  f  g  h  j  k  l  ^^
//   $  z  x  c  v  b  n  m  _  #

pub const LAYOUT_NUM: &'static str = &" 1 2 3 4 5 6 7 8 9 0 *
  ^ & @ \" # ( ) = $ % ^^
  $ , ; . : ! ? ' _ #";

//  1  2  3  4  5  6  7  8  9  0  *
//  ^  &  @  "  #  (  )  =  $  %  ^^
//   $  ,  ;  .  :  !  ?  '  _  #

// ! " # $ % & ' ( ) * + , - . /
// : ; < = > ?
// @
// [ \ ] ^ _
// `
// { | } ~ ?

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

pub enum InputState {
    Running(Keys),
    Updated,
    Validated,
    Overflow,
    NotForMe(Keys),
}

impl<const T: usize> Format for InputBuffer<T> {
    fn format(&self, _fmt: defmt::Formatter) {
        let t = intern!("{=[u8]:a}");
        defmt::export::istr(&t);
        let len = self.len();
        defmt::export::usize(&(len + 2));
        for i in 0..self.cursor {
            defmt::export::u8(&self.buffer[i])
        }
        defmt::export::u8(&b'>');
        defmt::export::u8(&b'<');
        for i in self.cursor..len {
            defmt::export::u8(&self.buffer[i])
        }
    }
}

pub struct InputBuffer<const S: usize> {
    pub buffer: [u8; S],
    last: Keys,
    ready: bool,
    cursor: usize,
}

impl<const S: usize> InputBuffer<S> {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; S],
            last: Keys::none(),
            ready: true,
            cursor: 0,
        }
    }
    pub fn len(&self) -> usize {
        let mut len = 0;
        for c in self.buffer {
            if c != 0 {
                len += 1;
            } else {
                break;
            }
        }
        len
    }
    pub fn get_data(&self) -> &[u8] {
        &self.buffer[0..self.len()]
    }
    pub fn get_cursor(&self) -> usize {
        self.cursor
    }
    pub fn clear(&mut self) {
        self.buffer = [0u8; S];
        self.cursor = 0;
    }
    pub fn process_input(&mut self, key: Keys) -> InputState {
        let mut ret = InputState::Running(key);
        if key != self.last {
            if key.contains(Keys::Star) {
                let key = key.xor(Keys::Star);
                match key {
                    Keys::ShiftR => {
                        if self.cursor == 0 {
                            ret = InputState::Overflow;
                        } else {
                            for i in self.cursor..S {
                                self.buffer[i - 1] = self.buffer[i];
                            }
                            self.cursor -= 1;
                            self.buffer[S - 1] = 0;
                            ret = InputState::Updated;
                        }
                        //_ = str.pop();
                        //info!("{}", str.as_str());
                    }
                    Keys::Q => {
                        //Left
                        if self.cursor == 0 {
                            ret = InputState::Overflow;
                        } else {
                            self.cursor -= 1;
                            ret = InputState::Updated;
                        }
                    }
                    Keys::E => {
                        //Right
                        if self.cursor >= S || self.buffer[self.cursor] == 0 {
                            ret = InputState::Overflow;
                        } else {
                            self.cursor += 1;
                            ret = InputState::Updated;
                        }
                    }
                    val => {
                        ret = InputState::NotForMe(val);
                    }
                }
            } else {
                let car = key.get_one_char();

                if car.is_none() {
                    self.ready = true;
                }
                if let (Some(car), true) = (car, self.ready) {
                    if self.cursor >= S || self.buffer[S - 1] != 0 {
                        ret = InputState::Overflow;
                    } else {
                        for i in (self.cursor..S - 1).rev() {
                            self.buffer[i + 1] = self.buffer[i];
                        }
                        self.buffer[self.cursor] = car as u8;
                        self.cursor += 1;
                        ret = InputState::Updated;
                    }
                    //_ = str.push(car);
                    //info!("{}", str.as_str());
                    self.ready = false;
                } else if self.ready && key == Keys::Sharp {
                    //info!("SENDING {}", str.as_str());
                    ret = InputState::Validated;
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
        }
        self.last = key;
        ret
    }
}

impl Keys {
    #[allow(non_upper_case_globals)]
    pub const Shift: Keys = Keys::ShiftR.or(Keys::ShiftL);
    #[allow(non_upper_case_globals)]
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

        _ = self.clk.set_low();
        _ = self.latch.set_low();
        cortex_m::asm::delay(10);
        _ = self.latch.set_high();

        for _ in 0..bits {
            v <<= one;
            cortex_m::asm::delay(10);

            if self.data.is_high().unwrap() {
                v += one;
            }
            _ = self.clk.set_high();
            cortex_m::asm::delay(10);

            _ = self.clk.set_low();
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
