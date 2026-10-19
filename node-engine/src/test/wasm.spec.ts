import { expect, test } from 'vitest'

import { UnleashEngine } from '../..'

test('evaluates context properties and keeps the feature name separate', () => {
  const engine = new UnleashEngine()
  try {
    engine.takeState(
      JSON.stringify({
        version: 1,
        features: [
          {
            name: 'context-feature',
            enabled: true,
            impressionData: true,
            strategies: [
              {
                name: 'default',
                constraints: [{ contextName: 'plan', operator: 'IN', values: ['42'] }],
              },
            ],
          },
        ],
      }),
    )

    expect(engine.isEnabled('context-feature', { plan: 42, toggleName: 'missing' })).toEqual({
      enabled: true,
      impressionData: true,
    })
    expect(engine.isEnabled('context-feature', { plan: 42, properties: { plan: 'other' } }).enabled).toBe(false)
    expect(engine.isEnabled('missing').enabled).toBeUndefined()
  } finally {
    engine.destroy()
  }
})

test('supports random rollout and the current time in WASM', () => {
  const engine = new UnleashEngine()
  try {
    engine.takeState(
      JSON.stringify({
        version: 1,
        features: [
          {
            name: 'wasm-runtime',
            enabled: true,
            strategies: [
              {
                name: 'flexibleRollout',
                parameters: { rollout: '100', stickiness: 'random', groupId: 'wasm-runtime' },
                constraints: [{ contextName: 'currentTime', operator: 'DATE_AFTER', value: '2000-01-01T00:00:00Z' }],
              },
            ],
          },
        ],
      }),
    )

    expect(engine.isEnabled('wasm-runtime').enabled).toBe(true)
  } finally {
    engine.destroy()
  }
})

test('throws for invalid state and rejects calls after freeing the engine', () => {
  const engine = new UnleashEngine()
  try {
    expect(() => engine.takeState('invalid JSON')).toThrow(/Failed to parse client features/)
  } finally {
    engine.destroy()
  }
  engine.destroy()

  expect(() => engine.isEnabled('missing')).toThrow('Engine state is not initialized')
  expect(() => engine.takeState('{}')).toThrow('Engine state is not initialized')
})
