use esp_idf_svc::sys::{esp_rom_gpio_pad_select_gpio, gpio_set_direction, gpio_set_level};

pub struct ControlGpio {
    pin: i32,
    mode: u32
}

impl ControlGpio {
    pub fn new<'a>(pin: i32, mode: u32) -> Self {

        if pin > 30 {
            panic!("Please, digit a valid pin!");
        }
        
        if mode > 7 {
            panic!("Please, digit a valid mode for pin!");
        }

        let pin_driver = ControlGpio { pin, mode };

        unsafe {
            esp_rom_gpio_pad_select_gpio(pin_driver.pin as u32);
            gpio_set_direction(pin_driver.pin, pin_driver.mode);
        }

        pin_driver
    }

    pub fn set_value(&self, value: u32) -> Result<(), &str> {

        if value > 10000 {
            return Err("Please, digit a valid value for pin!");
        }

        unsafe {
            gpio_set_level(self.pin, value);
        }

        Ok(())
    }

    pub fn change_mode(&mut self, mode: u32) -> Result<(), &str> {
        
        if mode > 7 {
            return Err("Please, digit a valid mode for pin!");
        }

        self.mode = mode;
        
        Ok(())
    }
}