import {
  type ClientFeatures,
  type ClientFeaturesDelta,
  type EnabledResult,
  getSpecVersion,
  getSupportedStrategies,
  getYggdrasilVersion,
  type KnownFeatures,
  type MetricsResult,
  type VariantResult,
  WasmInterface,
} from '../wasm/wasm-interface.js';
import { type CustomStrategy, CustomStrategyHandler } from './custom-strategies.js';

export type {
  ClientFeatures,
  ClientFeaturesDelta,
  CollectedMetric,
  EnabledResult,
  KnownFeatures,
  MetricBucket,
  MetricsResult,
  StateWarnings,
  ToggleDefinition,
  VariantResult,
} from '../wasm/wasm-interface.js';

export interface Context {
  [key: string]: unknown;
  properties?: Record<string, unknown>;
}

export { getSpecVersion, getSupportedStrategies, getYggdrasilVersion };

export class UnleashEngine {
  wasmInterface: WasmInterface | null;
  customStrategyHandler: CustomStrategyHandler;

  constructor() {
    this.wasmInterface = new WasmInterface();
    this.customStrategyHandler = new CustomStrategyHandler();
  }

  destroy(): void {
    if (!this.wasmInterface) return;
    this.wasmInterface.destroy();
    this.wasmInterface.free();
    this.wasmInterface = null;
  }

  takeState(state: ClientFeatures | ClientFeaturesDelta): string | null {
    this.assertInitialized();
    const warnings = this.wasmInterface.takeState(JSON.stringify(state));
    this.customStrategyHandler.updateStrategies(state);
    return warnings;
  }

  isEnabled(featureName: string, context?: Context): EnabledResult {
    this.assertInitialized();
    try {
      const customResults = this.customStrategyHandler.evaluateCustomStrategies(
        featureName,
        context ?? {},
      );
      return this.wasmInterface.isEnabled(
        JSON.stringify({ ...context, toggleName: featureName, externalResults: customResults }),
      );
    } catch (_: unknown) {
      return {
        enabled: undefined,
        impressionData: false,
      };
    }
  }

  getVariant(featureName: string, context?: Context): VariantResult {
    this.assertInitialized();
    try {
      const customResults = this.customStrategyHandler.evaluateCustomStrategies(
        featureName,
        context ?? {},
      );
      return this.wasmInterface.getVariant(
        JSON.stringify({ ...context, toggleName: featureName, externalResults: customResults }),
      );
    } catch (_: unknown) {
      return {
        variant: undefined,
        impressionData: false,
      };
    }
  }

  checkVariant(featureName: string, context?: Context): VariantResult {
    this.assertInitialized();
    try {
      const customResults = this.customStrategyHandler.evaluateCustomStrategies(
        featureName,
        context ?? {},
      );
      return this.wasmInterface.checkVariant(
        JSON.stringify({ ...context, toggleName: featureName, externalResults: customResults }),
      );
    } catch (_: unknown) {
      return {
        variant: undefined,
        impressionData: false,
      };
    }
  }

  count(featureName: string, isEnabled: boolean): void {
    this.assertInitialized();
    this.wasmInterface.count(featureName, isEnabled);
  }

  countVariant(featureName: string, variantName: string): void {
    this.assertInitialized();
    this.wasmInterface.countVariant(featureName, variantName);
  }

  getMetrics(): MetricsResult {
    this.assertInitialized();
    return this.wasmInterface.getMetrics();
  }

  restoreMetrics(metrics: MetricsResult): void {
    this.assertInitialized();
    this.wasmInterface.restoreMetrics(JSON.stringify(metrics));
  }

  getState(): ClientFeatures {
    this.assertInitialized();
    return this.wasmInterface.getState();
  }

  listFeatures(): KnownFeatures {
    this.assertInitialized();
    return this.wasmInterface.listFeatures();
  }

  defineCounter(name: string, helpText: string): void {
    this.assertInitialized();
    this.wasmInterface.defineCounter(name, helpText);
  }

  defineGauge(name: string, helpText: string): void {
    this.assertInitialized();
    this.wasmInterface.defineGauge(name, helpText);
  }

  defineHistogram(name: string, helpText: string, buckets?: number[]): void {
    this.assertInitialized();
    this.wasmInterface.defineHistogram(name, helpText, new Float64Array(buckets ?? []));
  }

  incrementCounter(name: string, value: number = 1): void {
    this.assertInitialized();

    if (!Number.isSafeInteger(value)) {
      throw new RangeError('Counter increment must be a safe integer');
    }

    this.wasmInterface.incrementCounter(name, BigInt(value));
  }

  updateGauge(name: string, value: number): void {
    this.assertInitialized();
    this.wasmInterface.updateGauge(name, value);
  }

  observeHistogram(name: string, value: number): void {
    this.assertInitialized();
    this.wasmInterface.observeHistogram(name, value);
  }

  registerCustomStrategies(strategies: CustomStrategy[]): void {
    this.assertInitialized();
    this.customStrategyHandler.registerCustomStrategies(strategies);
  }

  private assertInitialized(): asserts this is this & { wasmInterface: WasmInterface } {
    if (!this.wasmInterface) throw new Error('Engine state is not initialized');
  }
}
