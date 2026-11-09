// SPDX-License-Identifier: MIT OR Apache-2.0
// Excel custom functions over the Dimvexa WASM build (ADR-015). No conversion logic lives here.
// Loaded after dimvexa-glue.js (defines `wasm_bindgen`) and dimvexa-wasm-b64.js (defines
// `DIMVEXA_WASM_B64`) by functions.html.
/* global wasm_bindgen, DIMVEXA_WASM_B64, CustomFunctions */

function base64ToBytes(b64) {
  // Own decoder: the custom functions runtime is a bare JavaScript environment, so do not rely
  // on `atob` being there.
  const abc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const lookup = new Uint8Array(128);
  for (let i = 0; i < abc.length; i++) lookup[abc.charCodeAt(i)] = i;
  const pad = b64.endsWith("==") ? 2 : b64.endsWith("=") ? 1 : 0;
  const out = new Uint8Array((b64.length * 3) / 4 - pad);
  let o = 0;
  for (let i = 0; i < b64.length; i += 4) {
    const n =
      (lookup[b64.charCodeAt(i)] << 18) |
      (lookup[b64.charCodeAt(i + 1)] << 12) |
      (lookup[b64.charCodeAt(i + 2)] << 6) |
      lookup[b64.charCodeAt(i + 3)];
    if (o < out.length) out[o++] = (n >> 16) & 255;
    if (o < out.length) out[o++] = (n >> 8) & 255;
    if (o < out.length) out[o++] = n & 255;
  }
  return out;
}

let ready = null;
function init() {
  if (ready === null) {
    ready = wasm_bindgen({ module_or_path: base64ToBytes(DIMVEXA_WASM_B64) });
  }
  return ready;
}

// A refusal is an Excel error with the stable Dimvexa code in the message, never a guess.
function toExcelError(e) {
  const code = e && e.code ? e.code : "internal";
  let message = "dimvexa " + code + ": " + (e && e.message ? e.message : String(e));
  if (e && e.suggestions && e.suggestions.length > 0) {
    message += " (did you mean " + Array.from(e.suggestions).join(", ") + "?)";
  }
  const kind = code === "overflow" ? CustomFunctions.ErrorCode.invalidNumber
    : CustomFunctions.ErrorCode.invalidValue;
  return new CustomFunctions.Error(kind, message);
}

/** @customfunction CONVERT */
async function convert(value, from, to) {
  await init();
  try {
    return wasm_bindgen.convert(value, from, to);
  } catch (e) {
    throw toExcelError(e);
  }
}

/** @customfunction VERSION */
async function version() {
  await init();
  return "dimvexa data " + wasm_bindgen.dataVersion() + ", " + wasm_bindgen.unitCount() + " units";
}

CustomFunctions.associate("CONVERT", convert);
CustomFunctions.associate("VERSION", version);
