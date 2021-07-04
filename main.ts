import { deferred } from "https://deno.land/std@0.100.0/async/deferred.ts";
import { assert } from "https://deno.land/std@0.100.0/testing/asserts.ts";

const promise = deferred();
const workerOptions: WorkerOptions = { type: "module" };
const w = new Worker(
  new URL("worker.js", import.meta.url).href,
  workerOptions,
);
const sab = new SharedArrayBuffer(8);
const i32 = new Int32Array(sab);
i32[0] = 1;

const result = (Atomics as any).waitAsync(i32, 0, 1, 1000);

if (result.value === 'not-equal') {
  // The value in the SharedArrayBuffer was not the expected one.
  throw "The initial value should be 1."
} else {
  assert(result.value instanceof Promise);

  result.value.then((value: any) => {
    console.log(value);
    promise.resolve();
  });
}

w.postMessage(sab);

// Uncomment section below to make code work:
/*
setTimeout(() => {
  console.log(i32[0]); // 123 -> the value has changed an this thread is active, so why not resolve waitAsync?
  console.log(result.value);
}, 600);
*/

await promise;

w.terminate();
