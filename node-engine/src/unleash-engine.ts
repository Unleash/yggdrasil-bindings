import { type EnabledResult, type VariantResult, WasmInterface } from '../wasm/wasm-interface.js';

export type {
  EnabledResult,
  Payload,
  StateWarnings,
  Variant,
  VariantResult,
} from '../wasm/wasm-interface.js';

export interface Context {
  [key: string]: unknown;
  properties?: Record<string, unknown>;
}

export class UnleashEngine {
  wasmInterface: WasmInterface | null;

  constructor() {
    this.wasmInterface = new WasmInterface();
  }

  destroy(): void {
    if (!this.wasmInterface) return;
    this.wasmInterface.destroy();
    this.wasmInterface.free();
    this.wasmInterface = null;
  }

  takeState(rawState: string): string | null {
    this.assertInitialized();
    return this.wasmInterface.takeState(rawState);
  }

  isEnabled(featureName: string, context?: Context): EnabledResult {
    this.assertInitialized();
    return this.wasmInterface.isEnabled(JSON.stringify({ ...context, toggleName: featureName }));
  }

  getVariant(featureName: string, context?: Context): VariantResult {
    this.assertInitialized();
    return this.wasmInterface.getVariant(JSON.stringify({ ...context, toggleName: featureName }));
  }

  count(featureName: string, isEnabled: boolean): void {
    this.assertInitialized();
    this.wasmInterface.count(featureName, isEnabled);
  }

  countVariant(featureName: string, variantName: string): void {
    this.assertInitialized();
    this.wasmInterface.countVariant(featureName, variantName);
  }

  private assertInitialized(): asserts this is this & { wasmInterface: WasmInterface } {
    if (!this.wasmInterface) throw new Error('Engine state is not initialized');
  }
}
