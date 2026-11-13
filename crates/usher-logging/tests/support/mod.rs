//! A logging-owned capture backend used only by public contract tests.
use std::sync::{Arc, Mutex};
use usher_logging::{
    Output,
    runtime::{Health, LogWriter, Record},
};
#[derive(Clone, Default)]
pub struct Capture(pub Arc<Mutex<Vec<(Output, Record)>>>);
impl Capture {
    pub fn records(&self) -> Vec<(Output, Record)> {
        self.0.lock().unwrap().clone()
    }
}
impl LogWriter for Capture {
    fn write(&mut self, output: Output, record: &Record) -> Result<(), Health> {
        self.0.lock().unwrap().push((output, record.clone()));
        Ok(())
    }
    fn flush(&mut self) -> Result<(), Health> {
        Ok(())
    }
}
pub struct FailingFile {
    pub capture: Capture,
}
impl LogWriter for FailingFile {
    fn write(&mut self, output: Output, record: &Record) -> Result<(), Health> {
        if output == Output::File {
            Err(Health::WriteFailed)
        } else {
            self.capture.write(output, record)
        }
    }
    fn flush(&mut self) -> Result<(), Health> {
        Ok(())
    }
}

pub struct GatedCapture {
    pub capture: Capture,
    pub started: std::sync::mpsc::Sender<()>,
    pub release: std::sync::mpsc::Receiver<()>,
}
impl LogWriter for GatedCapture {
    fn write(&mut self, output: Output, record: &Record) -> Result<(), Health> {
        if record.event == "hold" {
            self.started.send(()).unwrap();
            self.release
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }
        self.capture.write(output, record)
    }
    fn flush(&mut self) -> Result<(), Health> {
        Ok(())
    }
}
