import {
  type ClientFeature,
  type ClientFeatures,
  type ClientFeaturesDelta,
  getSupportedStrategies,
} from '../wasm/wasm-interface.js';
import type { Context } from './unleash-engine.js';

export interface CustomStrategy {
  name: string;
  isEnabled(parameters: Record<string, string>, context: Context): boolean;
}

interface StrategyDefinition {
  name: string;
  parameters?: Record<string, string>;
}

export class CustomStrategyHandler {
  private readonly standardStrategies = new Set(getSupportedStrategies());
  private customStrategyDefinitions = new Map<string, StrategyDefinition[]>();
  private readonly customStrategyImplementations = new Map<string, CustomStrategy>();

  updateStrategies(state: ClientFeatures | ClientFeaturesDelta): void {
    const definitions =
      'features' in state
        ? new Map<string, StrategyDefinition[]>()
        : new Map(this.customStrategyDefinitions);

    if ('features' in state) {
      for (const feature of state.features) {
        this.updateFeature(definitions, feature);
      }
    } else {
      for (const event of state.events) {
        switch (event.type) {
          case 'hydration':
            definitions.clear();
            for (const feature of event.features) {
              this.updateFeature(definitions, feature);
            }
            break;
          case 'feature-updated':
            this.updateFeature(definitions, event.feature);
            break;
          case 'feature-removed':
            definitions.delete(event.featureName);
            break;
        }
      }
    }

    this.customStrategyDefinitions = definitions;
  }

  registerCustomStrategies(strategies: CustomStrategy[]): void {
    for (const strategy of strategies) {
      if (
        strategy == null ||
        typeof strategy.name !== 'string' ||
        typeof strategy.isEnabled !== 'function'
      ) {
        throw new TypeError(
          'Invalid strategy object. Must have a string name and an isEnabled method.',
        );
      }
      this.customStrategyImplementations.set(strategy.name, strategy);
    }
  }

  evaluateCustomStrategies(toggleName: string, context: Context): Record<string, boolean> {
    const results: Record<string, boolean> = {};
    const strategies = this.customStrategyDefinitions.get(toggleName) ?? [];

    strategies.forEach((strategy, index) => {
      const implementation = this.customStrategyImplementations.get(strategy.name);
      results[`customStrategy${index + 1}`] =
        implementation?.isEnabled(strategy.parameters ?? {}, context) ?? false;
    });

    return results;
  }

  private updateFeature(
    definitions: Map<string, StrategyDefinition[]>,
    feature: ClientFeature,
  ): void {
    const strategies = (feature.strategies ?? []).filter(
      (strategy) => !this.standardStrategies.has(strategy.name),
    );

    if (strategies.length > 0) {
      definitions.set(
        feature.name,
        strategies.map((strategy) => ({
          name: strategy.name,
          parameters: strategy.parameters ?? {},
        })),
      );
    } else {
      definitions.delete(feature.name);
    }
  }
}
