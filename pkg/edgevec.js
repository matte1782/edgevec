import * as wasm from "./edgevec_bg.wasm";
export * from "./edgevec_bg.js";
import { __wbg_set_wasm } from "./edgevec_bg.js";
__wbg_set_wasm(wasm);