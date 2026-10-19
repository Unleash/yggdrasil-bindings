import { expect, test } from 'vitest'

import { UnleashEngine } from '../..'

const smokeTestState = {
  "version": 1,
  "features": [
    {
      "name": "demo-feature",
      "description": "Enabled toggle",
      "enabled": true,
      "strategies": [{
        "name": "default"
      }]
    }
  ]
}

test('checks a feature against a new engine', () => {
  const engine = new UnleashEngine()
  const warnings = engine.takeState(JSON.stringify(smokeTestState))

  const result = engine.isEnabled('demo-feature')

  expect(warnings).toBeNull()
  expect(typeof result.impressionData).toBe('boolean')
  expect(result.enabled).toBe(true)
})

test('checks a feature with an explicit empty context', () => {
  const engine = new UnleashEngine()
  engine.takeState(JSON.stringify(smokeTestState))

  const result = engine.isEnabled('demo-feature', {})

  expect(result.enabled).toBe(true)
  expect(result.impressionData).toBe(false)
})
