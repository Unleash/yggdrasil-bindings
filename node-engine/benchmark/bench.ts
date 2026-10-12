import { Bench } from 'tinybench'

import { NodeEngine } from '../index.js'

const b = new Bench()
const engine = new NodeEngine()

b.add('NodeEngine.isEnabled', () => {
  engine.isEnabled('demo-feature')
})

await b.run()

console.table(b.table())
