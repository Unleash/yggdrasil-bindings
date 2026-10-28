import { expect, expectTypeOf, test } from 'vitest';

import { type EnabledResult, UnleashEngine, type VariantResult } from '../..';
import type { WasmInterface } from '../../wasm/wasm-interface.js';

type IsAny<T> = 0 extends 1 & T ? true : false;

test('preserves Rust-generated response types through the public wrapper', () => {
  expectTypeOf<IsAny<ReturnType<WasmInterface['isEnabled']>>>().toEqualTypeOf<false>();
  expectTypeOf<IsAny<ReturnType<WasmInterface['getVariant']>>>().toEqualTypeOf<false>();
  expectTypeOf<IsAny<ReturnType<WasmInterface['takeState']>>>().toEqualTypeOf<false>();
  expectTypeOf<ReturnType<UnleashEngine['isEnabled']>>().toEqualTypeOf<EnabledResult>();
  expectTypeOf<ReturnType<UnleashEngine['getVariant']>>().toEqualTypeOf<VariantResult>();
  expectTypeOf<ReturnType<UnleashEngine['takeState']>>().toEqualTypeOf<string | null>();
});

test('serializes variant payloads and missing variants with the existing shape', () => {
  const engine = new UnleashEngine();
  try {
    expect(
      engine.takeState({
        version: 1,
        features: [
          {
            name: 'variant-feature',
            enabled: true,
            impressionData: true,
            strategies: [{ name: 'default' }],
            variants: [
              {
                name: 'blue',
                weight: 1000,
                payload: { type: 'json', value: '{"plan":"pro"}' },
              },
            ],
          },
        ],
      }),
    ).toBeNull();

    expect(engine.getVariant('variant-feature', { userId: 'test-user' })).toEqual({
      variant: {
        name: 'blue',
        payload: { type: 'json', value: '{"plan":"pro"}' },
        enabled: true,
        featureEnabled: true,
      },
      impressionData: true,
    });
    expect(engine.getVariant('missing')).toEqual({
      variant: undefined,
      impressionData: false,
    });
    expect(engine.isEnabled('missing')).toEqual({ impressionData: false });
  } finally {
    engine.destroy();
  }
});

test('preserves warning strings from partially compiled state', () => {
  const engine = new UnleashEngine();
  try {
    const warnings = engine.takeState({
      version: 1,
      features: [
        {
          name: 'invalid-constraint',
          enabled: true,
          strategies: [
            {
              name: 'default',
              constraints: [{ contextName: 'currentTime', operator: 'DATE_AFTER' }],
            },
          ],
        },
      ],
    });

    expect(typeof warnings).toBe('string');
    expect(warnings).toContain('invalid-constraint');
  } finally {
    engine.destroy();
  }
});
