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

#[derive(Default)]
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
        todo!()
    }
}

pub struct Oled128x128<'a> {
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
            Text::new(&self.input, Point::new(0, 127), self.style).draw(display);
        }
        if self.title_modified {
            Rectangle::new(Point::new(0, 0), Size::new(128, 12))
                .into_styled(self.clear_style)
                .draw(display);
            //.unwrap();

            self.title_modified = false;
            Text::new(&self.title, Point::new(0, 12), self.style).draw(display);
        }
        if self.body_modified {
            Rectangle::new(Point::new(0, 12), Size::new(128, 128 - 12 * 2))
                .into_styled(self.clear_style)
                .draw(display);
            self.body_modified = false;
            for (i, line) in self.body.iter().enumerate() {
                let y = i as i32 * 12 + 12;
                let u = Text::new(&line.up, Point::new(0, y), self.style_small).draw(display);
                let d = Text::new(&line.down, Point::new(0, y + 6), self.style_small).draw(display);
                let x = if let (Ok(u), Ok(d)) = (u, d) {
                    i32::max(u.x, d.x)
                } else {
                    0
                };
                Text::new(&line.body, Point::new(x, y), self.style).draw(display);
            }
        }
    }
}
