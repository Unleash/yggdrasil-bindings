import { NodeEngine } from '../wasm/node_engine.js';

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

export class Wrapper {
  nodeEngine: NodeEngine | null;

  constructor() {
    this.nodeEngine = new NodeEngine();
  }

  destroy(): void {
    if (!this.nodeEngine) return;
    this.nodeEngine.destroy();
    this.nodeEngine.free();
    this.nodeEngine = null;
  }

  takeState(rawState: string): string | null {
    this.assertInitialized();
    return this.nodeEngine.takeState(rawState);
  }

  isEnabled(featureName: string, context?: Context): EnabledResult {
    this.assertInitialized();
    return this.nodeEngine.isEnabled(JSON.stringify({ ...context, toggleName: featureName }));
  }

  getVariant(featureName: string, context?: Context): Variant | undefined {
    this.assertInitialized();
    return this.nodeEngine.getVariant(JSON.stringify({ ...context, toggleName: featureName }));
  }

  count(featureName: string, isEnabled: boolean): void {
    this.assertInitialized();
    this.nodeEngine.count(featureName, isEnabled);
  }

  countVariant(featureName: string, variantName: string): void {
    this.assertInitialized();
    this.nodeEngine.countVariant(featureName, variantName);
  }

  private assertInitialized(): asserts this is this & { nodeEngine: NodeEngine } {
    if (!this.nodeEngine) throw new Error('Engine state is not initialized');
  }
}
