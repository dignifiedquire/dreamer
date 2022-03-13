use async_std::task;
use epi::backend::RepaintSignal;
use std::{future::Future, sync::Arc};

pub struct Scheduler {
    repaint_signal: Option<Arc<dyn RepaintSignal>>,
}

#[derive(Clone, PartialEq)]
pub struct Event {}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            repaint_signal: None,
        }
    }

    pub fn init(&mut self, frame: &epi::Frame) {
        self.repaint_signal = Some(frame.0.lock().unwrap().repaint_signal.clone());
    }

    pub fn spawn<F, T>(&self, fut: F)
    where
        F: FnOnce(Option<Arc<dyn RepaintSignal>>) -> T + Send + 'static,
        T: Future<Output = ()> + Send + 'static,
    {
        let repaint_signal = self.repaint_signal.clone();
        task::spawn(async move {
            fut(repaint_signal.clone()).await;
            if let Some(r) = repaint_signal {
                r.request_repaint();
            }
        });
    }

    pub fn poll(&mut self) {}
}
