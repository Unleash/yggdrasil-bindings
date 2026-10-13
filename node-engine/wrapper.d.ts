import type { NodeEngine } from './wasm/node_engine'

export interface EnabledResult {
  enabled?: boolean
  impressionData: boolean
}

export interface Context {
  [key: string]: unknown
  properties?: Record<string, unknown>
}

export declare class Wrapper {
  constructor()
  nodeEngine: NodeEngine | null
  destroy(): void
  takeState(rawState: string): string | null
  isEnabled(featureName: string, context?: Context): EnabledResult
}
