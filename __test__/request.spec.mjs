import test from 'ava'

import { Request, Headers } from '../index.js'

test('minimum construction requirements', (t) => {
  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php'
  })

  t.is(req.method, 'GET')
  t.is(req.url, 'http://example.com/test.php')
  t.assert(req.body instanceof Buffer)
  t.is(req.body.length, 0)
  t.assert(req.headers instanceof Headers)
  t.is(req.headers.size, 0)
})

test('full construction', (t) => {
  const req = new Request({
    method: 'POST',
    url: 'http://example.com/test.php',
    body: Buffer.from('Hello, from Node.js!'),
    headers: {
      'Content-Type': ['application/json'],
      'X-Custom-Header': ['CustomValue']
    }
  })

  t.is(req.method, 'POST')
  t.is(req.url, 'http://example.com/test.php')
  t.assert(req.body instanceof Buffer)
  t.is(req.body.toString('utf8'), 'Hello, from Node.js!')
  t.assert(req.headers instanceof Headers)
  t.is(req.headers.size, 2)
  t.is(req.headers.get('Content-Type'), 'application/json')
  t.is(req.headers.get('X-Custom-Header'), 'CustomValue')
})
