import { Bench } from 'tinybench'

import { add } from '../wrapper.js'

const b = new Bench({
  time: 1_000,
  warmupTime: 500,
})

b.add('add', () => {
  add(20, 22)
})

await b.run()

console.table(b.table())
