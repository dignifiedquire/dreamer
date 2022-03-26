use async_std::task;
use epi::Frame;
use std::{future::Future, sync::Arc};

pub struct Scheduler {
    repaint_signal: Option<Frame>,
}

#[derive(Clone, PartialEq)]
pub struct Event {}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            repaint_signal: None,
        }
    }

    pub fn init(&mut self, frame: &Frame) {
        self.repaint_signal = Some(frame.clone());
    }

    pub fn spawn<F, T>(&self, fut: F)
    where
        F: FnOnce(Option<Frame>) -> T + Send + 'static,
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
