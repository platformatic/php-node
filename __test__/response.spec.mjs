import test from 'ava'

import { Response, Headers } from '../index.js'

test('Minimal response construction', (t) => {
  const res = new Response({
    status: 200
  })

  t.is(res.status, 200)
  t.assert(res.headers instanceof Headers)
  t.assert(res.body instanceof Buffer)
  t.deepEqual(res.body.toString(), '')
  t.assert(res.log instanceof Buffer)
  t.deepEqual(res.log.toString(), '')
  t.is(res.exception, null)
})

test('Full Response construction', (t) => {
  const json = JSON.stringify({
    message: 'Hello, world!'
  })

  const res = new Response({
    status: 200,
    headers: {
      'Content-Type': ['application/json'],
      'X-Custom-Header': ['CustomValue']
    },
    body: Buffer.from(json),
    log: Buffer.from('Hello, from error_log!'),
    exception: 'Hello, from PHP!'
  })

  t.is(res.status, 200)
  t.assert(res.headers instanceof Headers)
  t.deepEqual(res.headers.get('Content-Type'), 'application/json')
  t.assert(res.body instanceof Buffer)
  t.deepEqual(res.body.toString(), json)
  t.assert(res.log instanceof Buffer)
  t.deepEqual(res.log.toString(), 'Hello, from error_log!')
  t.deepEqual(res.exception, 'Hello, from PHP!')
})
