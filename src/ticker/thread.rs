use std::{thread::JoinHandle, time::Duration};

use super::Error;

pub struct TickerImpl {
    close_channel: std::sync::mpsc::Sender<()>,
    handle: JoinHandle<()>,
    start_instant: std::time::Instant,
}

impl TickerImpl {
    pub fn new(frequency: u32, engine: wasmtime::Engine) -> Result<Self, crate::ticker::Error> {
        let sleep_duration = Duration::from_secs_f64(1f64 / f64::from(frequency));
        let (close_channel, close_receiver) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || loop {
            match close_receiver.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => (),
            }

            spin_sleep::sleep(sleep_duration);

            match close_receiver.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => engine.increment_epoch(),
            }
        });
        Ok(Self {
            close_channel,
            handle,
            start_instant: std::time::Instant::now(),
        })
    }

    pub fn duration(&self) -> Duration {
        self.start_instant.elapsed()
    }

    pub fn end(self) -> Result<(), crate::ticker::Error> {
        let send_result = self.close_channel.send(());
        let join_result = self.handle.join();

        if send_result.is_err() || join_result.is_err() {
            return Err(Error::UnregisterError);
        }

        Ok(())
    }
}
