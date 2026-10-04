// Run Rust's single-threaded WASI test harness without filesystem access.
import { readFile } from 'node:fs/promises';
import { WASI } from 'node:wasi';

const args = process.argv.slice(2);
const wasi = new WASI({
  version: 'preview1',
  args,
  env: process.env,
  returnOnExit: true,
});
const module = await WebAssembly.compile(await readFile(args[0]));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exitCode = wasi.start(instance);
