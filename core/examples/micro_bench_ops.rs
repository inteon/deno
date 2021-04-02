// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate log;

use deno_core::error::custom_error;
use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::OpState;
use std::env;
use std::time::Instant;

pub type StartTime = Instant;

struct Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &log::Metadata) -> bool {
    metadata.level() <= log::max_level()
  }

  fn log(&self, record: &log::Record) {
    if self.enabled(record.metadata()) {
      println!("{} - {}", record.level(), record.args());
    }
  }

  fn flush(&self) {}
}

fn create_js_runtime() -> JsRuntime {
  let mut runtime = JsRuntime::new(Default::default());
  {
    let op_state = runtime.op_state();
    let mut state = op_state.borrow_mut();
    state.put::<StartTime>(StartTime::now());
  }

  // Fixed size sync msgs
  runtime.register_op(
    "json_sync",
    deno_core::json_op_sync(|_state, arg: u32, _buf| {
      debug!("json_sync");
      Ok(arg)
    }),
  );
  runtime.register_op(
    "json_sync_err",
    deno_core::json_op_sync(|_state, _arg: u32, _buf| -> Result<u32, AnyError> {
      debug!("json_sync_err");
      Err(custom_error(
        "MicroBenchError",
        "This error is used for testing the ops performance in case of an error.",
      ))
    }),
  );
  runtime.register_op(
    "bin_sync",
    deno_core::bin_op_sync(|_state, arg: u32, _buf| {
      debug!("bin_sync");
      Ok(arg)
    }),
  );
  runtime.register_op(
    "bin_sync_err",
    deno_core::bin_op_sync(|_state, _arg: u32, _buf| -> Result<u32, AnyError> {
      debug!("bin_sync_err");
      Err(custom_error(
        "MicroBenchError",
        "This error is used for testing the ops performance in case of an error.",
      ))
    }),
  );

  // Fixed size async msgs
  runtime.register_op(
    "json_async",
    deno_core::json_op_async(|_state, arg: u32, _buf| async move {
      debug!("json_async");
      Ok(arg)
    }),
  );
  runtime.register_op(
    "json_async_err",
    deno_core::json_op_async(|_state, _arg: u32, _buf| async {
      debug!("json_async_err");
      Err(custom_error(
        "MicroBenchError",
        "This error is used for testing the ops performance in case of an error.",
      )) as Result<u32, AnyError>
    }),
  );
  runtime.register_op(
    "bin_async",
    deno_core::bin_op_async(|_state, arg: u32, _buf| async move {
      debug!("bin_async");
      Ok(arg)
    }),
  );
  runtime.register_op(
    "bin_async_err",
    deno_core::bin_op_async(|_state, _arg: u32, _buf| async {
      debug!("bin_async_err");
      Err(custom_error(
        "MicroBenchError",
        "This error is used for testing the ops performance in case of an error.",
      )) as Result<u32, AnyError>
    }),
  );

  runtime.register_op("op_now", deno_core::bin_op_sync(|op_state: &mut OpState, _arg: u32, buf| {
    let start_time = op_state.borrow::<StartTime>();
    let seconds = start_time.elapsed().as_secs();
    let subsec_nanos = start_time.elapsed().subsec_nanos() as f64;

    let result = (seconds * 1_000) as f64 + (subsec_nanos / 1_000_000.0);

    (&mut buf.unwrap()).copy_from_slice(&result.to_be_bytes());

    Ok(0)
  }));

  runtime
}

fn main() {
  log::set_logger(&Logger).unwrap();
  log::set_max_level(
    env::args()
      .find(|a| a == "-D")
      .map(|_| log::LevelFilter::Debug)
      .unwrap_or(log::LevelFilter::Warn),
  );

  // NOTE: `--help` arg will display V8 help and exit
  deno_core::v8_set_flags(env::args().collect());

  let mut js_runtime = create_js_runtime();
  let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();

  let silent = env::args()
    .find(|a| a == "-s")
    .map(|_| true)
    .unwrap_or(false);

  js_runtime
    .execute(
      "envset.js",
      if silent {
        "function getShouldPrint() { return false; }"
      } else {
        "function getShouldPrint() { return true; }"
      },
    )
    .unwrap();

  runtime.block_on(js_runtime.run_event_loop()).unwrap();

  js_runtime
    .execute("micro_bench_ops.js", include_str!("micro_bench_ops.js"))
    .unwrap();

  runtime.block_on(js_runtime.run_event_loop()).unwrap();
}
