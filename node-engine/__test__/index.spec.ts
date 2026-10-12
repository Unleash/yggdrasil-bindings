import test from 'ava'

import { NodeEngine } from '../index'

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

test('checks a feature against a new engine', (t) => {
  const engine = new NodeEngine()
  const warnings = engine.takeState(JSON.stringify(smokeTestState))

  const result = engine.isEnabled('demo-feature')

  t.true(warnings === null)
  t.is(typeof result.impressionData, 'boolean')
  t.true(result.enabled)
})
