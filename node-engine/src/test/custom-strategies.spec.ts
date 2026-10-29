import { expect, test, vi } from 'vitest';
import { CustomStrategyHandler } from '../custom-strategies.js';
import type { ClientFeature, ClientFeatures, Strategy } from '../../wasm/wasm-interface.js';

const strategy = (name: string, parameters?: Record<string, string>): Strategy => ({
  name,
  parameters,
  variants: undefined,
});

const customFeature = (name: string): ClientFeature => ({
  name,
  enabled: true,
  strategies: [strategy('custom', { group: 'staff' })],
});

const snapshot = (features: ClientFeature[]): ClientFeatures => ({
  version: 1,
  query: undefined,
  features,
});

function createHandler() {
  const handler = new CustomStrategyHandler();
  handler.registerCustomStrategies([{ name: 'custom', isEnabled: () => true }]);
  return handler;
}

test('preserves method receivers and allows replacing implementations', () => {
  const handler = new CustomStrategyHandler();
  handler.updateStrategies(snapshot([customFeature('demo')]));
  handler.registerCustomStrategies([
    {
      name: 'custom',
      isEnabled() {
        return this.name === 'custom';
      },
    },
  ]);
  expect(handler.evaluateCustomStrategies('demo', {})).toEqual({ customStrategy1: true });
  handler.registerCustomStrategies([{ name: 'custom', isEnabled: () => false }]);
  expect(handler.evaluateCustomStrategies('demo', {})).toEqual({ customStrategy1: false });
});

test('full snapshots replace definitions without dropping implementations', () => {
  const handler = createHandler();
  handler.updateStrategies(snapshot([customFeature('old')]));
  handler.updateStrategies(snapshot([customFeature('new')]));
  expect(handler.evaluateCustomStrategies('old', {})).toEqual({});
  expect(handler.evaluateCustomStrategies('new', {})).toEqual({ customStrategy1: true });
  handler.updateStrategies(snapshot([]));
  expect(handler.evaluateCustomStrategies('new', {})).toEqual({});
});

test('delta updates preserve unchanged features and remove stale definitions', () => {
  const handler = createHandler();
  handler.updateStrategies(
    snapshot([customFeature('unchanged'), customFeature('removed'), customFeature('now-standard')]),
  );
  handler.updateStrategies({
    events: [
      { type: 'feature-updated', eventId: 1, feature: customFeature('new') },
      { type: 'feature-removed', eventId: 2, featureName: 'removed', project: 'default' },
      { type: 'feature-updated', eventId: 3, feature: { name: 'now-standard', enabled: true } },
      { type: 'segment-updated', eventId: 4, segment: { id: 1, constraints: [] } },
    ],
  });
  expect(handler.evaluateCustomStrategies('unchanged', {})).toEqual({ customStrategy1: true });
  expect(handler.evaluateCustomStrategies('new', {})).toEqual({ customStrategy1: true });
  expect(handler.evaluateCustomStrategies('removed', {})).toEqual({});
  expect(handler.evaluateCustomStrategies('now-standard', {})).toEqual({});
});

test('hydration replaces definitions and subsequent events apply in order', () => {
  const handler = createHandler();
  handler.updateStrategies(snapshot([customFeature('old')]));
  handler.updateStrategies({
    events: [
      { type: 'hydration', eventId: 1, features: [customFeature('hydrated')], segments: [] },
      { type: 'feature-updated', eventId: 2, feature: customFeature('new') },
      { type: 'feature-removed', eventId: 3, featureName: 'hydrated', project: 'default' },
    ],
  });
  expect(handler.evaluateCustomStrategies('old', {})).toEqual({});
  expect(handler.evaluateCustomStrategies('hydrated', {})).toEqual({});
  expect(handler.evaluateCustomStrategies('new', {})).toEqual({ customStrategy1: true });
});

test('rejects invalid implementations at runtime', () => {
  const handler = new CustomStrategyHandler();
  // JS consumers can bypass the TypeScript interface.
  // @ts-expect-error Missing isEnabled method.
  expect(() => handler.registerCustomStrategies([{ name: 'invalid' }])).toThrow(TypeError);
});
