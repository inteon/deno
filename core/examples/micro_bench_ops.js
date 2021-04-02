// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// deno-lint-ignore-file no-empty

const NR_OP_CALLS = 100000;

function assert(cond) {
  if (!cond) {
    throw Error("assert");
  }
}

// Bench functions
function binBenchSync(opName) {
  for (let i = 0; i < NR_OP_CALLS; i++) {
    Deno.core.binOpSync(opName, 42);
  }
}

function binBenchSyncErr(opName) {
  for (let i = 0; i < NR_OP_CALLS; i++) {
    try {
      Deno.core.binOpSync(opName, 42);
    } catch {}
  }
}

async function binBenchAsync(opName, binSize) {
  assert(NR_OP_CALLS % binSize === 0);
  for (let i = 0; i < NR_OP_CALLS / binSize; i++) {
    const promises = [];
    for (let i = 0; i < binSize; i++) {
      promises.push(Deno.core.binOpAsync(opName, 42));
    }
    await Promise.allSettled(promises);
  }
}

function jsonBenchSync(opName) {
  for (let i = 0; i < NR_OP_CALLS; i++) {
    Deno.core.jsonOpSync(opName, 42);
  }
}

function jsonBenchSyncErr(opName) {
  for (let i = 0; i < NR_OP_CALLS; i++) {
    try {
      Deno.core.jsonOpSync(opName, 42);
    } catch {}
  }
}

async function jsonBenchAsync(opName, binSize) {
  assert(NR_OP_CALLS % binSize === 0);
  for (let i = 0; i < NR_OP_CALLS / binSize; i++) {
    const promises = [];
    for (let i = 0; i < binSize; i++) {
      promises.push(Deno.core.jsonOpAsync(opName, 42));
    }
    await Promise.allSettled(promises);
  }
}

// Time function
const nowBytes = new Uint8Array(8);
function opNow() {
  Deno.core.binOpSync("op_now", 0, nowBytes);
  return new DataView(nowBytes.buffer).getFloat64();
}

function formatNumber(nr) {
  return nr.toFixed(2).replace(/\d(?=(\d{3})+\.)/g, '$&,');  // 12,345.67
}

async function main() {
  Deno.core.ops();
  Deno.core.registerErrorClass("Error", Error);

  // deno-lint-ignore no-undef
  const shouldPrint = getShouldPrint();

  const benches = {
    "bin_sync": binBenchSync,
    "bin_sync_err": binBenchSyncErr,
    "bin_async": binBenchAsync,
    "bin_async_err": binBenchAsync,
    "json_sync": jsonBenchSync,
    "json_sync_err": jsonBenchSyncErr,
    "json_async": jsonBenchAsync,
    "json_async_err": jsonBenchAsync,
  }

  let results = {}
  // Sync bench
  for (const type of ["bin", "json"]) {
    for (const error of [false, true]) {
      const name = `${type}_sync${error && "_err" || ""}`;
      const start = opNow();
      benches[name](name);
      const end = opNow();
      results[name] = end - start;
    }
  }

  // Async bench
  for (const type of ["bin", "json"]) {
    for (const error of [false, true]) {
      for (const bin of [1, 10, 100, 1000]) {
        const name = `${type}_async${error && "_err" || ""}`;
        const start = opNow();
        await benches[name](name, bin);
        const end = opNow();
        results[`${name}_${bin}`] = end - start;
      }
    }
  }

  const opsPerSecResults = {};
  const microsecPerOpResults = {};
  for (let bench in results) {
    const opsPerSec = (NR_OP_CALLS * 1000) / results[bench];
    const microsecPerOp = 1000 * results[bench] / NR_OP_CALLS;

    if (shouldPrint) Deno.core.print(`${bench}: ${formatNumber(opsPerSec)} op/s - ${formatNumber(microsecPerOp)} µs/op\n`);

    opsPerSecResults[bench] = opsPerSec;
    microsecPerOpResults[bench] = microsecPerOp;
  }

  if (!shouldPrint) {
    Deno.core.print(JSON.stringify({
      "ops_per_sec": opsPerSecResults,
      "microsec_per_op": microsecPerOpResults,
    }));
  }
}

main();
