use stm32_eth::hal::gpio::{ErasedPin, Input};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    High,
    Low,
}

/// A button that removes bounces that happen within bounce_threshhold and
/// provides helper functions to detect rising and falling edges.
///
/// The time scale can be anything (e.g. Seconds vs Milliseconds), as long as it
/// is the same for `time` and `debounce_threshhold`.
pub struct DebouncedButton {
    pin: ErasedPin<Input>,
    debounce_threshhold: u64,
    last_time: u64,
    last_state: State,
}

impl DebouncedButton {
    /// Convenience function to convert the bool to the State enum
    fn get_state_static(pin: &ErasedPin<Input>) -> State {
        if pin.is_high() {
            State::High
        } else {
            State::Low
        }
    }

    /// Returns the current state of the pin without applying debouncing
    fn get_state(&self) -> State {
        Self::get_state_static(&self.pin)
    }

    pub fn new(pin: ErasedPin<Input>, debounce_threshhold: u64) -> Self {
        let initial_state = Self::get_state_static(&pin);
        Self {
            pin,
            debounce_threshhold,
            last_time: 0,
            last_state: initial_state,
        }
    }

    /// Returns the current state of the button  (high vs low) after removing
    /// bounces. Debouncing only happens if this is called, so it is probably
    /// not a good idea to only call this if the state currently is X.
    fn debounced_state(&mut self, time: u64) -> State {
        let state = self.get_state();
        // Only do something if the state changed
        if state != self.last_state {
            // Only change the state if we're past the bounce threshhold. If
            // we're not this change can be considered a bounce and we just
            // update the time.
            if time > self.last_time + self.debounce_threshhold {
                self.last_state = state
            }
            self.last_time = time;
        }
        self.last_state
    }

    /// Returns true if the pin goes from low to high (ignoring bounces)
    pub fn is_rising_edge(&mut self, time: u64) -> bool {
        let last_state = self.last_state;
        let new_state = self.debounced_state(time);
        last_state == State::Low && new_state == State::High
    }

    /// Returns true if the pin goes from high to low (ignoring bounces)
    pub fn is_falling_edge(&mut self, time: u64) -> bool {
        let last_state = self.last_state;
        let new_state = self.debounced_state(time);
        last_state == State::High && new_state == State::Low
    }
}
