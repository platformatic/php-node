import test from 'ava'

import { handleRequest } from '../index.js'

test('sum from native', (t) => {
  t.is(handleRequest(), "Hello, World!")
})
