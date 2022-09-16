use cortex_m::asm::delay;
use embedded_hal_02::digital::v2::OutputPin;

#[derive(Clone, Copy)]
pub enum Morse {
    Dash,
    Dot,
    Letter,
    Word,
}

impl Morse {
    pub fn time(&self) -> (u32, u32) {
        match self {
            Morse::Dash => (3, 1),
            Morse::Dot => (1, 1),
            Morse::Letter => (0, 2),
            Morse::Word => (0, 4),
        }
    }
}

static ms: u32 = 125000;

//light then dark
//Dash   ---_    3 1
//Dot    -_      1 1
//Letter __      0 2
//Word   ____  0 4

static L: Morse = Morse::Dash;
static O: Morse = Morse::Dot;
static P: Morse = Morse::Letter;
static S: Morse = Morse::Word;

static LA: [Morse; 2] = [O, L]; //A
static LB: [Morse; 4] = [L, O, O, O]; //B
static LC: [Morse; 4] = [L, O, L, O]; //C
static LD: [Morse; 3] = [L, O, O];
static LE: [Morse; 1] = [O];
static LF: [Morse; 4] = [O, O, L, O];
static LG: [Morse; 3] = [L, L, O];
static LH: [Morse; 4] = [O, O, O, O];
static LI: [Morse; 2] = [O, O];
static LJ: [Morse; 4] = [O, L, L, L];
static LK: [Morse; 3] = [L, O, L];
static LL: [Morse; 4] = [O, L, O, O];
static LM: [Morse; 2] = [L, L];
static LN: [Morse; 2] = [L, O];
static LO: [Morse; 3] = [L, L, L];
static LP: [Morse; 4] = [O, L, L, O];
static LQ: [Morse; 4] = [L, L, O, L];
static LR: [Morse; 3] = [O, L, O];
static LS: [Morse; 3] = [O, O, O];
static LT: [Morse; 1] = [L];
static LU: [Morse; 3] = [O, O, L];
static LV: [Morse; 4] = [O, O, O, L];
static LW: [Morse; 3] = [O, L, L];
static LX: [Morse; 4] = [L, O, O, L];
static LY: [Morse; 4] = [L, O, L, L];
static LZ: [Morse; 4] = [L, L, O, O];
static L1: [Morse; 5] = [O, L, L, L, L];
static L2: [Morse; 5] = [O, O, L, L, L];
static L3: [Morse; 5] = [O, O, O, L, L];
static L4: [Morse; 5] = [O, O, O, O, L];
static L5: [Morse; 5] = [O, O, O, O, O];
static L6: [Morse; 5] = [L, O, O, O, O];
static L7: [Morse; 5] = [L, L, O, O, O];
static L8: [Morse; 5] = [L, L, L, O, O];
static L9: [Morse; 5] = [L, L, L, L, O];
static L0: [Morse; 5] = [L, L, L, L, L];
static LPERIOD: [Morse; 6] = [O, L, O, L, O, L];
static LCOMMA: [Morse; 6] = [L, L, O, O, L, L];
static LQUESTION: [Morse; 6] = [O, O, L, L, O, O];
static LAPOSTROPHE: [Morse; 6] = [O, L, L, L, L, O];
static LSPACE: [Morse; 1] = [S];
static LERROR: [Morse; 3] = [S, O, S];

pub fn blink<P>(pin: &mut P, string: &str)
where
    P: OutputPin,
    P::Error: core::fmt::Debug,
{
    blink_iter(pin, string.chars())
}

pub fn blink_sign<P>(pin: &mut P, s: Morse)
where
    P: OutputPin,
    P::Error: core::fmt::Debug,
{
    let (h, l) = s.time();
    if h > 0 {
        pin.set_high().unwrap();
        delay(h * 200 * ms);
    }
    pin.set_low().unwrap();
    delay(l * 200 * ms)
}

pub fn blink_iter<P, I>(pin: &mut P, iter: I)
where
    P: OutputPin,
    I: IntoIterator<Item = char>,
    P::Error: core::fmt::Debug,
{
    for l in iter.into_iter() {
        let c = letter(l);
        for &sign in c {
            blink_sign(pin, sign)
        }
        blink_sign(pin, Morse::Letter)
    }
    blink_sign(pin, Morse::Word);
    blink_sign(pin, Morse::Word);
    blink_sign(pin, Morse::Word);
}

pub fn letter(c: char) -> &'static [Morse] {
    match c {
        'A' | 'a' => &LA,
        'B' | 'b' => &LB,
        'C' | 'c' => &LC,
        'D' | 'd' => &LD,
        'E' | 'e' => &LE,
        'F' | 'f' => &LF,
        'G' | 'g' => &LG,
        'H' | 'h' => &LH,
        'I' | 'i' => &LI,
        'J' | 'j' => &LJ,
        'K' | 'k' => &LK,
        'L' | 'l' => &LL,
        'M' | 'm' => &LM,
        'N' | 'n' => &LN,
        'O' | 'o' => &LO,
        'P' | 'p' => &LP,
        'Q' | 'q' => &LQ,
        'R' | 'r' => &LR,
        'S' | 's' => &LS,
        'T' | 't' => &LT,
        'U' | 'u' => &LU,
        'V' | 'v' => &LV,
        'W' | 'w' => &LW,
        'X' | 'x' => &LX,
        'Y' | 'y' => &LY,
        'Z' | 'z' => &LZ,
        '1' => &L1,
        '2' => &L2,
        '3' => &L3,
        '4' => &L4,
        '5' => &L5,
        '6' => &L6,
        '7' => &L7,
        '8' => &L8,
        '9' => &L9,
        '0' => &L0,
        ' ' => &LSPACE,
        '\'' => &LAPOSTROPHE,
        ',' => &LCOMMA,
        '.' => &LPERIOD,
        '?' => &LQUESTION,
        _ => &LERROR,
    }
}
