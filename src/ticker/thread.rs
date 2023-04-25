use std::time::Duration;

use crate::ENGINE;

use super::Error;

pub struct TickerImpl {
    close_channel: std::sync::mpsc::Sender<()>,
    start_instant: std::time::Instant,
}

impl TickerImpl {
    pub fn new(frequency: u32) -> Result<Self, crate::ticker::Error> {
        let sleep_duration = Duration::from_secs_f64(1f64 / f64::from(frequency));
        let (close_channel, close_receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || loop {
            match close_receiver.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => (),
            }
            spin_sleep::sleep(sleep_duration);
            if let Some(engine) = unsafe { ENGINE.as_ref() } {
                engine.increment_epoch();
            }
        });
        Ok(Self {
            close_channel,
            start_instant: std::time::Instant::now(),
        })
    }

    pub fn duration(&self) -> Duration {
        self.start_instant.elapsed()
    }

    pub fn end(self) -> Result<(), crate::ticker::Error> {
        self.close_channel
            .send(())
            .map_err(|_| Error::UnregisterError)?;
        Ok(())
    }
}
