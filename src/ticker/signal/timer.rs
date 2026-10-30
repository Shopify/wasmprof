use std::ptr::null_mut;
use std::time::{Duration, Instant};

use libc::{itimerval, setitimer, timeval, ITIMER_PROF};

pub struct Timer {
    pub start_instant: Instant,
}

impl Timer {
    pub fn new(frequency: u32) -> Timer {
        let time = 1f64 / f64::from(frequency);
        let duration = Duration::from_secs_f64(time);
        let it_interval = timeval {
            tv_sec: duration.as_secs().try_into().unwrap(),
            tv_usec: duration.subsec_micros() as _,
        };
        let it_value = it_interval;

        unsafe {
            setitimer(
                ITIMER_PROF,
                &itimerval {
                    it_interval,
                    it_value,
                },
                null_mut(),
            )
        };

        Timer {
            start_instant: Instant::now(),
        }
    }

    pub fn duration(&self) -> Duration {
        self.start_instant.elapsed()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let it_interval = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let it_value = it_interval;
        unsafe {
            setitimer(
                ITIMER_PROF,
                &itimerval {
                    it_interval,
                    it_value,
                },
                null_mut(),
            )
        };
    }
}
