// Runs a scenario through the WebAssembly module, as a browser would: node scripts/wasm-run.mjs
// <swat_wasm.wasm> <scenario.tar> <out.tar> [flags]. Prints the progress calls' count and
// run.json's size; the output tar holds the tables and run.json.
import { readFileSync, writeFileSync } from "node:fs";

const [wasmPath, inPath, outPath, flagsArg] = process.argv.slice(2);
let calls = 0;
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {
  env: { swat_progress: (done, days) => { calls += 1; if (calls % 20 === 0) console.log(`progress ${done}/${days}`); } },
});
const { memory, swat_alloc, swat_free, swat_run } = instance.exports;
const input = readFileSync(inPath);
const ptr = swat_alloc(input.length);
new Uint8Array(memory.buffer, ptr, input.length).set(input);
const res = swat_run(ptr, input.length, Number(flagsArg ?? 3));
swat_free(ptr, input.length);
const len = new DataView(memory.buffer).getUint32(res, true);
const out = new Uint8Array(memory.buffer, res + 4, len).slice();
swat_free(res, len + 4);
writeFileSync(outPath, out);
console.log(`${calls} progress calls, ${len} bytes out`);
