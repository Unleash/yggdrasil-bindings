import { expect, test } from 'vitest'

import { add } from '../..'

test('adds numbers across the WASM boundary', () => {
  expect(add(20, 22)).toBe(42)
  expect(add(-5, 2)).toBe(-3)
})
