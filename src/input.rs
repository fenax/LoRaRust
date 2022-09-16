use embedded_hal_02::digital::v2::InputPin;

pub struct Button<P>
where
    P: InputPin,
{
    button: P,
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
