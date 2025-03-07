import test from 'ava'

import { Php, Request } from '../index.js'

test('sum from native', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `
      http_response_code(400);

      echo phpinfo();
    `
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php',
    headers: {
      'Content-Type': 'application/json',
      'Content-Length': 13
    },
    body: 'Hello, World!'
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 400)
  t.is(res.body, "wat")
})
