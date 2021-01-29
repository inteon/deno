// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use crate::worker_communication::WorkerHandle;
use crate::worker_communication::WorkerEvent;
use deno_core::error::generic_error;
use deno_core::error::AnyError;
use deno_core::futures::channel::mpsc;
use deno_core::futures::stream::StreamExt;
use deno_core::serde::Deserialize;
use deno_core::serde_json;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::ZeroCopyBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Deserialize)]
struct HostUnhandledErrorArgs {
  message: String,
}

pub fn init(
  rt: &mut deno_core::JsRuntime,
  sender: mpsc::Sender<WorkerEvent>,
  receiver: Arc<AsyncMutex<mpsc::Receiver<Box<[u8]>>>>,
  worker_handle: WorkerHandle,
) {
  super::reg_json_async(
    rt,
    "op_worker_get_message",
    move |_state, _args, _bufs| {
      let receiver_clone = receiver.clone();
      op_worker_get_message(receiver_clone)
    },
  );

  let sender_clone1 = sender.clone();
  super::reg_json_sync(
    rt,
    "op_worker_post_message",
    move |_state, _args, bufs| {
      let sender_clone = sender_clone1.clone();
      op_worker_post_message(bufs, sender_clone)
    },
  );

  let sender_clone1 = sender.clone();
  super::reg_json_sync(rt, "op_worker_close", move |_state, _args, _bufs| {
    let sender_clone = sender_clone1.clone();
    op_worker_close(&worker_handle, sender_clone)
  });

  super::reg_json_sync(
    rt,
    "op_worker_unhandled_error",
    move |_state, args, _zero_copy| {
      let sender_clone = sender.clone();
      op_worker_unhandled_error(args, sender_clone)
    },
  );
}

fn op_worker_unhandled_error(
  args: Value,
  mut sender: mpsc::Sender<WorkerEvent>,
) -> Result<Value, AnyError> {
  let args: HostUnhandledErrorArgs = serde_json::from_value(args)?;
  sender
    .try_send(WorkerEvent::Error(generic_error(args.message)))
    .expect("Failed to propagate error event to parent worker");
  Ok(json!(true))
}

/// Get message from host as worker
async fn op_worker_get_message(
  receiver_ref: Arc<AsyncMutex<mpsc::Receiver<Box<[u8]>>>>,
) -> Result<Value, AnyError> {
  let mut receiver = receiver_ref.try_lock()?;
  let maybe_event = receiver.next().await;

  if let Some(event) = maybe_event {
    let json = String::from_utf8(event.to_vec())?;

    return Ok(json!({ "json": json }));
  }

  Ok(json!({ "type": "close" }))
}

/// Post message to host as worker
fn op_worker_post_message(
  bufs: &mut [ZeroCopyBuf],
  mut sender: mpsc::Sender<WorkerEvent>,
) -> Result<Value, AnyError> {
  assert_eq!(bufs.len(), 1, "Invalid number of arguments");

  let msg_buf: Box<[u8]> = (*bufs[0]).into();
  sender
    .try_send(WorkerEvent::Message(msg_buf))
    .expect("Failed to post message to host");
  Ok(json!({}))
}

fn op_worker_close(
  worker_handle: &WorkerHandle,
  mut sender: mpsc::Sender<WorkerEvent>,
) -> Result<Value, AnyError> {
  // Notify parent that we're finished
  sender.close_channel();
  // Terminate execution of current worker
  worker_handle.terminate();
  Ok(json!({}))
}
