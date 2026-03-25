#!/usr/bin/env -S node --experimental-strip-types

import process from "node:process";
import { runCli } from "./wendao_gateway_openapi_benchmark.ts";

runCli(process.argv.slice(2)).then((code) => {
  process.exit(code);
});
