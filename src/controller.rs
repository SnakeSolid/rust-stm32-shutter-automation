use stm32f1xx_hal::gpio::Output;
use stm32f1xx_hal::gpio::Pin;

pub trait UpDownPin {
    fn disable(&mut self);
    fn enable(&mut self);
    fn is_active(&self) -> bool;
}

impl<MODE, CR, const P: char, const N: u8> UpDownPin for Pin<Output<MODE>, CR, P, N> {
    fn disable(&mut self) {
        self.set_low();
    }

    fn enable(&mut self) {
        self.set_high();
    }

    fn is_active(&self) -> bool {
        self.is_set_high()
    }
}

pub struct ControlChannel<'a> {
    pin_up: &'a mut dyn UpDownPin,
    pin_down: &'a mut dyn UpDownPin,
    time_up: Option<u32>,
    time_down: Option<u32>,
    limit_up: Option<u32>,
    limit_down: Option<u32>,
}

impl<'a> ControlChannel<'a> {
    pub fn new<'b>(
        pin_up: &'b mut dyn UpDownPin,
        pin_down: &'b mut dyn UpDownPin,
    ) -> ControlChannel<'b> {
        ControlChannel {
            pin_up,
            pin_down,
            time_up: None,
            time_down: None,
            limit_up: None,
            limit_down: None,
        }
    }

    pub fn stop(&mut self) {
        self.pin_up.disable();
        self.pin_down.disable();
        self.time_up = None;
        self.time_down = None;
    }

    pub fn up(&mut self) {
        self.pin_up.enable();
        self.pin_down.disable();
        self.time_up = self.limit_up;
        self.time_down = None;
    }

    pub fn down(&mut self) {
        self.pin_up.disable();
        self.pin_down.enable();
        self.time_up = None;
        self.time_down = self.limit_down;
    }

    pub fn set_limit(&mut self, limit_up: Option<u32>, limit_down: Option<u32>) {
        self.limit_up = limit_up;
        self.limit_down = limit_down;

        if self.pin_up.is_active() {
            self.time_up = self.limit_up;
        }

        if self.pin_down.is_active() {
            self.time_down = self.limit_down;
        }
    }

    pub fn update(&mut self, delta: u32) {
        match self.time_up.as_mut() {
            Some(time) if *time > delta => *time -= delta,
            Some(_) => {
                self.pin_up.disable();
                self.time_up = None;
            }
            None => {}
        }

        match self.time_down.as_mut() {
            Some(time) if *time > delta => *time -= delta,
            Some(_) => {
                self.pin_down.disable();
                self.time_down = None;
            }
            None => {}
        }
    }

    pub fn is_active(&self) -> bool {
        self.pin_up.is_active() || self.pin_down.is_active()
    }
}

pub struct Controller<'a, const N: usize> {
    channels: [ControlChannel<'a>; N],
}

impl<'a, const N: usize> Controller<'a, N> {
    pub fn new<'b>(channels: [ControlChannel<'b>; N]) -> Controller<'b, N> {
        Controller { channels }
    }

    pub fn stop_all(&mut self) {
        for channel in &mut self.channels {
            channel.stop();
        }
    }

    pub fn stop(&mut self, index: usize) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.stop();
        }
    }

    pub fn up(&mut self, index: usize) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.up();
        }
    }

    pub fn down(&mut self, index: usize) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.down();
        }
    }

    pub fn limit(&mut self, index: usize, up_limit: Option<u32>, down_limit: Option<u32>) {
        if let Some(channel) = self.channels.get_mut(index) {
            channel.set_limit(up_limit, down_limit);
        }
    }

    pub fn update(&mut self, delta: u32) {
        for channel in &mut self.channels {
            channel.update(delta);
        }
    }

    pub fn is_active(&self) -> bool {
        for channel in &self.channels {
            if channel.is_active() {
                return true;
            }
        }

        false
    }
}
