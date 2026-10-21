// Runs the smoke module in Node: node wasm-smoke/smoke.mjs <path to .wasm> [expected result]
// The expected result is 2 for the default build and 3 for a build with feature `sign`.
import { readFileSync } from "node:fs";
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]), {});
const result = instance.exports.smoke();
console.log(`smoke() = ${result}`);
process.exit(result === Number(process.argv[3] ?? 2) ? 0 : 1);
