// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_core::error::AnyError;
use deno_core::futures::channel::mpsc;
use deno_core::futures::stream::StreamExt;
use deno_core::v8;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

  /// Events sent from child to parent
  pub enum WorkerEvent {
    Message(Box<[u8]>),
    Error(AnyError),
    TerminalError(AnyError),
  }
  
  // Channels used for communication with worker's parent
    pub struct WorkerChannelsInternal {
    pub sender: mpsc::Sender<WorkerEvent>,
    pub receiver: Arc<AsyncMutex<mpsc::Receiver<Box<[u8]>>>>,
    pub(crate) terminate_rx: mpsc::Receiver<()>,
  }
  
  /// Parent uses this struct to communicate with worker and terminate it,
  /// while worker uses it only to finish execution on `self.close()`.
  #[derive(Clone)]
  pub struct WorkerHandle {
    pub sender: mpsc::Sender<Box<[u8]>>,
    pub receiver: Arc<AsyncMutex<mpsc::Receiver<WorkerEvent>>>,
    pub(crate) terminate_tx: mpsc::Sender<()>,
    pub(crate) terminated: Arc<AtomicBool>,
    pub(crate) isolate_handle: v8::IsolateHandle,
  }
  
  impl WorkerHandle {
    /// Post message to worker as a host.
    pub fn post_message(&self, buf: Box<[u8]>) -> Result<(), AnyError> {
      let mut sender = self.sender.clone();
      sender.try_send(buf)?;
      Ok(())
    }
  
    /// Get the event with lock.
    /// Return error if more than one listener tries to get event
    pub async fn get_event(&self) -> Result<Option<WorkerEvent>, AnyError> {
      let mut receiver = self.receiver.try_lock()?;
      Ok(receiver.next().await)
    }
  
    pub fn terminate(&self) {
      // This function can be called multiple times by whomever holds
      // the handle. However only a single "termination" should occur so
      // we need a guard here.
      let already_terminated = self.terminated.swap(true, Ordering::SeqCst);
  
      if !already_terminated {
        self.isolate_handle.terminate_execution();
        let mut sender = self.terminate_tx.clone();
        // This call should be infallible hence the `expect`.
        // This might change in the future.
        sender.try_send(()).expect("Failed to terminate");
      }
    }
  }
  
  pub(crate) fn create_channels(
    isolate_handle: v8::IsolateHandle
  ) -> (WorkerChannelsInternal, WorkerHandle) {
    let (terminate_tx, terminate_rx) = mpsc::channel::<()>(1);
    let (in_tx, in_rx) = mpsc::channel::<Box<[u8]>>(1);
    let (out_tx, out_rx) = mpsc::channel::<WorkerEvent>(1);
    let internal_channels = WorkerChannelsInternal {
      sender: out_tx,
      receiver: Arc::new(AsyncMutex::new(in_rx)),
      terminate_rx,
    };
    let external_channels = WorkerHandle {
      sender: in_tx,
      receiver: Arc::new(AsyncMutex::new(out_rx)),
      terminated: Arc::new(AtomicBool::new(false)),
      terminate_tx,
      isolate_handle,
    };
    (internal_channels, external_channels)
  }
