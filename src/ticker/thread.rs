use std::time::SystemTime;

use crate::ENGINE;

use super::{Error, ReportTiming};

pub struct TickerImpl {
    close_channel: std::sync::mpsc::Sender<()>,
    start_time: SystemTime,
    start_instant: std::time::Instant,
    frequency: i32,
}

impl TickerImpl {
    pub fn new(frequency: i32) -> Result<Self, crate::ticker::Error> {
        let (close_channel, close_receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || loop {
            match close_receiver.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => (),
            }
            std::thread::sleep(std::time::Duration::from_millis(1000 / frequency as u64));
            if let Some(engine) = unsafe { ENGINE.as_ref() } {
                engine.increment_epoch();
            }
        });
        Ok(Self {
            close_channel,
            start_time: SystemTime::now(),
            start_instant: std::time::Instant::now(),
            frequency,
        })
    }

    pub fn timing(&self) -> ReportTiming {
        ReportTiming {
            frequency: self.frequency,
            start_time: self.start_time,
            duration: self.start_instant.elapsed(),
        }
    }

    pub fn end(self) -> Result<(), crate::ticker::Error> {
        self.close_channel
            .send(())
            .map_err(|_| Error::UnregisterError)?;
        Ok(())
    }
}
