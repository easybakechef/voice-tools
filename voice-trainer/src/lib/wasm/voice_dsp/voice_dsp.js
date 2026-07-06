/* @ts-self-types="./voice_dsp.d.ts" */
import * as wasm from "./voice_dsp_bg.wasm";
import { __wbg_set_wasm } from "./voice_dsp_bg.js";

__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
export {
    detect_pitch
} from "./voice_dsp_bg.js";
