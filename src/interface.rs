#![allow(dead_code)]

use core::i32::MAX;

use embedded_graphics::{
    mono_font::{ascii::FONT_4X6, ascii::FONT_6X12, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::*,
};
use heapless::String;
use numtoa::NumToA;

#[derive(Default, Clone, PartialEq)]
pub struct LogLine {
    up: String<6>,
    down: String<6>,
    body: String<22>,
}

pub trait Interface {
    fn set_title(&mut self, title: &[u8]);
    fn set_input(&mut self, input: &[u8], cursor: usize);
    fn add_log(&mut self, body: &[u8], snr: Option<i16>, rssi: Option<i16>);
}

const SMALL_WIDTH: usize = 4;
const BIG_WIDTH: usize = 6;
const DISPLAY_WIDTH: usize = 128;
impl Interface for Oled128x128<'_> {
    fn set_title(&mut self, title: &[u8]) {
        self.title.clear();
        self.title
            .push_str(unsafe { core::str::from_utf8_unchecked(title) });
        self.title_modified = true;
    }

    fn set_input(&mut self, input: &[u8], cursor: usize) {
        self.input.clear();
        self.input
            .push_str(unsafe { core::str::from_utf8_unchecked(input) });
        self.input_modified = true;
    }

    fn add_log(&mut self, body: &[u8], snr: Option<i16>, rssi: Option<i16>) {
        let mut line = LogLine::default();
        let mut push_line = |line: &mut LogLine| {
            self.body_modified = true;
            for i in 1..self.body.len() {
                self.body[i - 1] = self.body[i].clone()
            }
            self.body[self.body.len() - 1] = line.clone();
            *line = LogLine::default();
        };
        if let Some(snr) = snr {
            let mut str_buff = [0u8; 6];
            let text = snr.numtoa_str(10, &mut str_buff);
            line.up.push_str(&text).unwrap();
        }
        if let Some(rssi) = rssi {
            let mut str_buff = [0u8; 6];
            let text = rssi.numtoa_str(10, &mut str_buff);
            line.down.push_str(&text).unwrap();
        }
        let prefix = line.up.len().max(line.down.len()) * SMALL_WIDTH;
        let mut available = (DISPLAY_WIDTH - prefix) / BIG_WIDTH;
        if let Ok(s) = core::str::from_utf8(body) {
            s.chars().for_each(|c| {
                if c == '\r' || c == '\n' {
                    push_line(&mut line);
                    available = DISPLAY_WIDTH / BIG_WIDTH;
                } else {
                    if available == 0 {
                        push_line(&mut line);
                        available = DISPLAY_WIDTH / BIG_WIDTH;
                    }
                    line.body.push(c);
                    available -= 1;
                }
            });
        } else {
            line.body.push_str("__UNPARSABLE__").unwrap();
        }
        if line != LogLine::default() {
            push_line(&mut line);
        }
    }
}

pub struct Oled128x128<'a> {
    text_style: TextStyle,
    style: MonoTextStyle<'a, BinaryColor>,
    style_small: MonoTextStyle<'a, BinaryColor>,
    clear_style: PrimitiveStyle<BinaryColor>,
    title: String<22>,
    body: [LogLine; 8],
    input: String<22>,
    cursor: usize,
    title_modified: bool,
    body_modified: bool,
    input_modified: bool,
}

impl Oled128x128<'_> {
    pub fn new() -> Self {
        Self {
            text_style: TextStyleBuilder::new().baseline(Baseline::Top).build(),
            style: MonoTextStyle::new(&FONT_6X12, BinaryColor::On),
            style_small: MonoTextStyle::new(&FONT_4X6, BinaryColor::On),
            clear_style: PrimitiveStyleBuilder::new()
                .fill_color(BinaryColor::Off)
                .build(),
            title: String::default(),
            body: Default::default(),
            input: String::default(),
            cursor: 0,
            title_modified: false,
            body_modified: false,
            input_modified: false,
        }
    }
    pub fn draw(&mut self, display: &mut impl DrawTarget<Color = BinaryColor>) {
        if self.input_modified {
            Rectangle::new(Point::new(0, 116), Size::new(128, 12))
                .into_styled(self.clear_style)
                .draw(display);
            //.unwrap();

            self.input_modified = false;
            Text::with_text_style(&self.input, Point::new(0, 116), self.style, self.text_style)
                .draw(display);
        }
        if self.title_modified {
            Rectangle::new(Point::new(0, 0), Size::new(128, 12))
                .into_styled(self.clear_style)
                .draw(display);
            //.unwrap();

            self.title_modified = false;
            Text::with_text_style(&self.title, Point::new(0, 0), self.style, self.text_style)
                .draw(display);
        }
        if self.body_modified {
            Rectangle::new(Point::new(0, 12), Size::new(128, 128 - 12 * 2))
                .into_styled(self.clear_style)
                .draw(display);
            self.body_modified = false;
            for (i, line) in self.body.iter().enumerate() {
                let y = i as i32 * 12 + 16;
                let u = Text::with_text_style(
                    &line.up,
                    Point::new(0, y),
                    self.style_small,
                    self.text_style,
                )
                .draw(display);
                let d = Text::with_text_style(
                    &line.down,
                    Point::new(0, y + 6),
                    self.style_small,
                    self.text_style,
                )
                .draw(display);
                let x = if let (Ok(u), Ok(d)) = (u, d) {
                    i32::max(u.x, d.x)
                } else {
                    0
                };
                Text::with_text_style(&line.body, Point::new(x, y), self.style, self.text_style)
                    .draw(display);
            }
        }
    }
}
