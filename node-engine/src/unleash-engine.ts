import { WasmInterface } from '../wasm/wasm-interface.js';

export enum PayloadType {
  STRING = 'string',
  JSON = 'json',
  CSV = 'csv',
  NUMBER = 'number',
}

export interface EnabledResult {
  enabled?: boolean;
  impressionData: boolean;
}

export interface Variant {
  name: string;
  payload?: { type: PayloadType; value: string };
  enabled: boolean;
  featureEnabled: boolean;
}

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

  getVariant(featureName: string, context?: Context): Variant | undefined {
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
