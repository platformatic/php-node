import test from 'ava'

import { Headers } from '../index.js'

test('initial map is empty', (t) => {
  const headers = new Headers()
  t.is(headers.size, 0)
})

test('setting a new header adjusts size and stores value', (t) => {
  const headers = new Headers()
  headers.set('Content-Type', 'application/json')
  t.is(headers.size, 1)
  t.assert(headers.has('Content-Type'))
  t.is(headers.get('Content-Type'), 'application/json')
})

test('only last set is used for get', (t) => {
  const headers = new Headers()
  headers.set('Content-Type', 'application/json')
  headers.add('Content-Type', 'text/html')
  t.is(headers.size, 1)
  t.assert(headers.has('Content-Type'))
  t.is(headers.get('Content-Type'), 'text/html')
})

test('adding a header with multiple values works and stores to a single entry', (t) => {
  const headers = new Headers()
  headers.add('Accept', 'application/json')
  headers.add('Accept', 'text/html')
  t.is(headers.size, 1)
  t.assert(headers.has('Accept'))
  t.deepEqual(headers.getAll('Accept'), ['application/json', 'text/html'])
  t.deepEqual(headers.getLine('Accept'), 'application/json,text/html')
  t.deepEqual(headers.get('Accept'), 'text/html')
})

test('deleting a header adjusts size and removes value', (t) => {
  const headers = new Headers()
  headers.set('Content-Type', 'application/json')
  headers.delete('Content-Type')
  t.is(headers.size, 0)
  t.assert(!headers.has('Content-Type'))
  t.is(headers.get('Content-Type'), null)
})

test('clearing headers resets size and removes all values', (t) => {
  const headers = new Headers()
  headers.set('Content-Type', 'application/json')
  headers.set('Accept', 'application/json')
  headers.clear()
  t.is(headers.size, 0)
  t.is(headers.get('Content-Type'), null)
  t.is(headers.get('Accept'), null)
})

test('includes iterator methods', (t) => {
  const headers = new Headers()
  headers.set('Content-Type', 'application/json')
  headers.set('Accept', 'application/json')

  const entries = Array.from(headers.entries())
    .sort((a, b) => a[0].localeCompare(b[0]))
  t.deepEqual(entries, [
    ['accept', 'application/json'],
    ['content-type',  'application/json']
  ])

  const keys = Array.from(headers.keys()).sort()
  t.deepEqual(keys, ['accept', 'content-type'])

  const values = Array.from(headers.values()).sort()
  t.deepEqual(values, ['application/json', 'application/json'])

  const seen = []
  headers.forEach((values, name, map) => {
    seen.push([name, values, map])
  })
  t.deepEqual(seen.sort((a, b) => a[0].localeCompare(b[0])), [
    ['accept', 'application/json', headers],
    ['content-type', 'application/json', headers]
  ])
})

test('construct from object', (t) => {
  const headers = new Headers({
    'Content-Type': 'application/json',
    'Accept': ['application/json', 'text/html']
  })
  t.assert(headers.has('Content-Type'))
  t.is(headers.get('Content-Type'), 'application/json')
  t.assert(headers.has('Accept'))
  t.deepEqual(headers.getAll('Accept'), ['application/json', 'text/html'])
})
