use std::io::Write;
use std::process::Child;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::JoinHandle;

/// A slow stdin reader must not block stdout, cancellation or deadline observation.
pub(super) struct StreamingStdin {
    sender: Option<SyncSender<Vec<u8>>>,
    pending: Option<Vec<u8>>,
    worker: Option<JoinHandle<()>>,
}

impl StreamingStdin {
    pub(super) fn start(child: &mut Child, initial: Option<&[u8]>, keep_open: bool) -> Self {
        let Some(mut pipe) = child.stdin.take() else {
            return Self {
                sender: None,
                pending: None,
                worker: None,
            };
        };
        let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(1);
        let initial = initial.map(<[u8]>::to_vec);
        let worker = std::thread::spawn(move || {
            if let Some(initial) = initial {
                if pipe.write_all(&initial).is_err() {
                    return;
                }
            }
            if keep_open {
                for data in receiver {
                    if pipe.write_all(&data).and_then(|_| pipe.flush()).is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            sender: keep_open.then_some(sender),
            pending: None,
            worker: Some(worker),
        }
    }

    pub(super) fn pump(&mut self) -> bool {
        let Some(data) = self.pending.take() else {
            return true;
        };
        let Some(sender) = &self.sender else {
            return true;
        };
        match sender.try_send(data) {
            Ok(()) => true,
            Err(TrySendError::Full(data)) => {
                self.pending = Some(data);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.sender = None;
                true
            }
        }
    }

    pub(super) fn write(&mut self, data: Vec<u8>) -> bool {
        self.pending = Some(data);
        self.pump()
    }

    pub(super) fn close(&mut self) {
        self.sender = None;
    }

    /// The child has exited or been killed before this join.
    pub(super) fn finish(mut self) {
        self.close();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
