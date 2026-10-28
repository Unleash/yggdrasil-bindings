import { expect, test, vi } from 'vitest';
import { UnleashEngine } from '../unleash-engine.js';

test('round-trips strategy parameter objects and evaluates custom strategies', () => {
  const engine = new UnleashEngine();
  const isEnabled = vi.fn((parameters) => parameters.group === 'staff');
  try {
    engine.registerCustomStrategies([{ name: 'custom', isEnabled }]);
    engine.takeState({
      version: 1,
      query: undefined,
      features: [
        {
          name: 'custom-feature',
          enabled: true,
          strategies: [{ name: 'custom', parameters: { group: 'staff' }, variants: undefined }],
        },
      ],
    });

    const state = engine.getState();
    expect(state.features[0].strategies?.[0].parameters).toStrictEqual({ group: 'staff' });
    engine.takeState(state);
    expect(engine.isEnabled('custom-feature').enabled).toBe(true);
    expect(isEnabled).toHaveBeenCalledWith({ group: 'staff' }, {});
  } finally {
    engine.destroy();
  }
});

test('serializes nested metrics maps as objects and restores them through JSON', () => {
  const engine = new UnleashEngine();
  try {
    engine.count('demo', true);
    engine.countVariant('demo', 'blue');
    engine.restoreMetrics({
      bucket: undefined,
      impactMetrics: [
        {
          name: 'requests',
          help: 'Request count',
          type: 'counter',
          samples: [{ labels: { region: 'eu' }, value: 3 }],
        },
      ],
    });

    const metrics = engine.getMetrics();
    expect(metrics.bucket?.toggles).toStrictEqual({
      demo: { yes: 1, no: 0, variants: { blue: 1 } },
    });
    expect(metrics.impactMetrics[0].samples).toStrictEqual([
      { labels: { region: 'eu' }, value: 3 },
    ]);

    engine.restoreMetrics(metrics);
    const restored = engine.getMetrics();
    expect(restored.bucket?.toggles).toStrictEqual(metrics.bucket?.toggles);
    expect(restored.impactMetrics).toStrictEqual(metrics.impactMetrics);
  } finally {
    engine.destroy();
  }
});
