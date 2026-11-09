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

// Start compiling the module when the script loads, so the first call usually finds it ready.
// Once it is ready the functions below return plain values (synchronous custom functions);
// only a call that arrives before that returns a Promise, which Excel shows as #BUSY!.
let ready = null;
let loaded = false;
function init() {
  if (ready === null) {
    ready = wasm_bindgen({ module_or_path: base64ToBytes(DIMVEXA_WASM_B64) }).then((r) => {
      loaded = true;
      return r;
    });
  }
  return ready;
}
function whenReady(fn) {
  return loaded ? fn() : init().then(fn);
}
init().catch(() => {});

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

// Excel passes undefined, null or an empty string for an omitted optional argument.
function given(arg) {
  return arg !== undefined && arg !== null && arg !== "";
}
function text(arg) {
  return given(arg) ? String(arg) : "";
}

/** @customfunction CONVERT */
function convert(value, from, to, density, config) {
  return whenReady(() => {
    try {
      return given(density) || given(config)
        ? wasm_bindgen.convertWith(value, from, to, text(density), text(config))
        : wasm_bindgen.convert(value, from, to);
    } catch (e) {
      throw toExcelError(e);
    }
  });
}

/** @customfunction CONVERTRANGE */
function convertRange(values, from, to, density, config) {
  return whenReady(() => convertRangeNow(values, from, to, density, config));
}

function convertRangeNow(values, from, to, density, config) {
  let conversion;
  try {
    // One composed conversion and one WASM call for the whole range.
    conversion = given(density) || given(config)
      ? wasm_bindgen.DimvexaConversion.withOptions(from, to, text(density), text(config))
      : new wasm_bindgen.DimvexaConversion(from, to);
    const rows = values.length;
    const cols = rows > 0 ? values[0].length : 0;
    const flat = new Float64Array(rows * cols);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) flat[r * cols + c] = values[r][c];
    }
    const out = conversion.applyBatch(flat);
    const result = [];
    for (let r = 0; r < rows; r++) {
      result.push(Array.from(out.subarray(r * cols, (r + 1) * cols)));
    }
    return result;
  } catch (e) {
    throw toExcelError(e);
  } finally {
    if (conversion) conversion.free();
  }
}

/** @customfunction VERSION */
function version() {
  return whenReady(
    () => "dimvexa data " + wasm_bindgen.dataVersion() + ", " + wasm_bindgen.unitCount() + " units",
  );
}

CustomFunctions.associate("CONVERT", convert);
CustomFunctions.associate("CONVERTRANGE", convertRange);
CustomFunctions.associate("VERSION", version);
