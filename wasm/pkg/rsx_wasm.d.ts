/* tslint:disable */
/* eslint-disable */
/**
*/
export enum LowInput {
  ButtonSelect = 0,
  ButtonL3 = 1,
  ButtonR3 = 2,
  ButtonStart = 3,
  ButtonUp = 4,
  ButtonRight = 5,
  ButtonDown = 6,
  ButtonLeft = 7,
}
/**
*/
export enum HighInput {
  ButtonL2 = 0,
  ButtonR2 = 1,
  ButtonL1 = 2,
  ButtonR1 = 3,
  ButtonTriangle = 4,
  ButtonCircle = 5,
  ButtonCross = 6,
  ButtonSquare = 7,
}
/**
*/
export class WasmEmulator {
  free(): void;
/**
* @param {Uint8Array} bios
* @param {Uint8Array} game_data
*/
  constructor(bios: Uint8Array, game_data: Uint8Array);
/**
*/
  run_frame(): void;
/**
* @returns {number}
*/
  get_framebuffer(): number;
/**
* @returns {number}
*/
  get_memory_card(): number;
/**
* @param {Uint8Array} memory_card
*/
  load_card(memory_card: Uint8Array): void;
/**
* @param {Float32Array} left
* @param {Float32Array} right
*/
  update_audio_buffers(left: Float32Array, right: Float32Array): void;
/**
* @returns {boolean}
*/
  toggle_digital_mode(): boolean;
/**
* @returns {boolean}
*/
  has_saved(): boolean;
/**
* @param {number} button
* @param {boolean} value
* @param {boolean} is_high_input
*/
  update_input(button: number, value: boolean, is_high_input: boolean): void;
/**
* @returns {number}
*/
  framebuffer_size(): number;
/**
* @returns {number}
*/
  memory_card_size(): number;
/**
* @returns {Uint32Array}
*/
  get_dimensions(): Uint32Array;
/**
* @param {number} val
*/
  update_leftx(val: number): void;
/**
* @param {number} val
*/
  update_lefty(val: number): void;
/**
* @param {number} val
*/
  update_rightx(val: number): void;
/**
* @param {number} val
*/
  update_righty(val: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmemulator_free: (a: number) => void;
  readonly wasmemulator_new: (a: number, b: number, c: number, d: number) => number;
  readonly wasmemulator_run_frame: (a: number) => void;
  readonly wasmemulator_get_framebuffer: (a: number) => number;
  readonly wasmemulator_get_memory_card: (a: number) => number;
  readonly wasmemulator_load_card: (a: number, b: number, c: number) => void;
  readonly wasmemulator_update_audio_buffers: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly wasmemulator_toggle_digital_mode: (a: number) => number;
  readonly wasmemulator_has_saved: (a: number) => number;
  readonly wasmemulator_update_input: (a: number, b: number, c: number, d: number) => void;
  readonly wasmemulator_framebuffer_size: (a: number) => number;
  readonly wasmemulator_memory_card_size: (a: number) => number;
  readonly wasmemulator_get_dimensions: (a: number, b: number) => void;
  readonly wasmemulator_update_leftx: (a: number, b: number) => void;
  readonly wasmemulator_update_lefty: (a: number, b: number) => void;
  readonly wasmemulator_update_rightx: (a: number, b: number) => void;
  readonly wasmemulator_update_righty: (a: number, b: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
