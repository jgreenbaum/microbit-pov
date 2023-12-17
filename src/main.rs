/***
 * "Persistence of Vision" demo on the MicroBit!
 * 
 * Have a little fun with your [MicroBit|https://microbit.org] and Rust!
 */

#![deny(unsafe_code)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use microbit::hal::gpio::{PushPull, Pin, Output};
use microbit_text::{scrolling_text::ScrollingStaticText, scrolling::Scrollable};
use rtt_target::rtt_init_print;
use panic_rtt_target as _;
#[cfg(feature = "profiling")]
use rtt_target::rprintln;

use core::f32::consts::PI;
use libm::atan2f;

#[cfg(feature = "v1")]
use microbit::{hal::twi, pac::twi0::frequency::FREQUENCY_A};

#[cfg(feature = "v2")]
use microbit::{hal::twim, pac::twim0::frequency::FREQUENCY_A};

use lsm303agr::{AccelOutputDataRate, Lsm303agr};


#[entry]
fn main() -> ! {
    // Enable init for rtt_panic_target
    rtt_init_print!();

    let mut board = microbit::Board::take().unwrap();

    #[cfg(feature = "v1")]
    let i2c = { twi::Twi::new(board.TWI0, board.i2c.into(), FREQUENCY_A::K100) };

    #[cfg(feature = "v2")]
    let i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };

    // Enable cycle counter for perf analysis
    board.DCB.enable_trace();
    board.DWT.enable_cycle_counter();

    let mut scroller = ScrollingStaticText::default();
    scroller.set_message(b"HELLO ");

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor.set_accel_odr(AccelOutputDataRate::Khz1_344).unwrap();

    // We are doing the top 1/5 of a circle
    let lightup_rads:f32 = (2.0*PI)/5.0;
    let min_circle_rad: f32 = (PI/2.0)-(lightup_rads/2.0);
    let max_circle_rad: f32 = (PI/2.0)+(lightup_rads/2.0);

    // Fire up the column pin
    board.display_pins.col3.set_low().unwrap();
    let mut row_pins = [board.display_pins.row1.degrade(),
                    board.display_pins.row2.degrade(),
                    board.display_pins.row3.degrade(),
                    board.display_pins.row4.degrade(),
                    board.display_pins.row5.degrade()];

    // Convenience function
    let set_pin = |p:&mut Pin<Output<PushPull>>, val| -> _ {
        if val > 0 {
            p.set_high().unwrap();
        } else {
            p.set_low().unwrap();
        }
    };
            
    loop {
        #[cfg(feature = "profiling")]
        let loop_start = board.DWT.cyccnt.read();
        
        // Read the latest data. Each loop iteration takes about 1 millisecond,
        // so given the sample rate is >1KHz it should be fresh.
        let accel_data = sensor.accel_data().unwrap();
        // Find our angle
        let theta = atan2f(accel_data.y as f32, accel_data.x as f32);
        // Compensate for looking at the "back" of the accelerometer
        let theta = if theta < 0.0 { -theta } else { theta };
        // Clamp to the display range
        let theta = if theta < min_circle_rad {
            min_circle_rad
        } else if theta > max_circle_rad {
            max_circle_rad
        } else {
            theta
        };
        // Find the column to display
        let frac = (theta-min_circle_rad)/lightup_rads;
        let num_scroll_pixels = scroller.length()*5;
        let scroll_index = ((num_scroll_pixels as f32) * frac) as usize;

        // Scroll to the column we want. No way to do this in microbit_text,
        // so just count it from the start.
        let scroll_state = scroller.state_mut();
        scroll_state.reset();
        for _i in 0..scroll_index { scroll_state.tick(); }

        // Now display the column's dots
        for row in 0..5 {
            set_pin(&mut row_pins[row], scroller.current_brightness_at(0, 4-row));
        }

        #[cfg(feature = "profiling")]
        {
            let loop_end = board.DWT.cyccnt.read();        
            let loop_time = ((loop_end-loop_start) as f32)/ 64000000.0;
            rprintln!("loop time = {}", loop_time);
        }
    }
}