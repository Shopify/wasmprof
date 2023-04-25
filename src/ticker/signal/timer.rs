use std::os::raw::c_int;
use std::ptr::null_mut;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone)]
struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Clone)]
struct Itimerval {
    pub it_interval: Timeval,
    pub it_value: Timeval,
}

extern "C" {
    fn setitimer(which: c_int, new_value: *mut Itimerval, old_value: *mut Itimerval) -> c_int;
}

const ITIMER_PROF: c_int = 2;

pub struct Timer {
    pub start_instant: Instant,
}

impl Timer {
    pub fn new(frequency: u32) -> Timer {
        let time = 1f64 / f64::from(frequency);
        let duration = Duration::from_secs_f64(time);
        let it_interval = Timeval {
            tv_sec: duration.as_secs().try_into().unwrap(),
            tv_usec: duration.subsec_micros().into(),
        };
        let it_value = it_interval.clone();

        unsafe {
            setitimer(
                ITIMER_PROF,
                &mut Itimerval {
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
        let it_interval = Timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let it_value = it_interval.clone();
        unsafe {
            setitimer(
                ITIMER_PROF,
                &mut Itimerval {
                    it_interval,
                    it_value,
                },
                null_mut(),
            )
        };
    }
}
