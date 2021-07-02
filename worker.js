globalThis.addEventListener("message", (e) => {
  const i32 = new Int32Array(e.data);

  i32[0] = 123;

  Atomics.notify(i32, 0);
});
