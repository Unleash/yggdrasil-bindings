import { Bench } from 'tinybench'

import { UnleashEngine } from '../lib/unleash-engine.js'

const b = new Bench({
  time: 1_000,
  warmupTime: 500,
})

const engine = new UnleashEngine()
const TOGGLE_NAME = 'bench-toggle';
const CONTEXT = { userId: 'bench-user' };

const bootstrap = {
  "version": 1,
  "features": [
    {
      "name": TOGGLE_NAME,
      "enabled": true,
      "strategies": [
        {
          "name": "default",
        }
      ]
    }
  ]
}

engine.takeState(JSON.stringify(bootstrap))

b.add('UnleashEngine.isEnabled', () => {
  engine.isEnabled(TOGGLE_NAME, CONTEXT)
})

await b.run()

console.table(b.table())
