import test from 'ava'

import { Php, Request } from '../index.js'

test('input/output streams work', async (t) => {
  const php = new Php({
    argv: process.argv,
    file: 'index.php',
    code: `
      if (file_get_contents('php://input') == 'Hello, from Node.js!') {
        echo 'Hello, from PHP!';
      }
    `
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php',
    body: Buffer.from('Hello, from Node.js!')
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.body.toString('utf8'), 'Hello, from PHP!')
})

test('logs work', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `
      error_log('Hello, from error_log!');
    `
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php'
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.log.toString('utf8'), 'Hello, from error_log!\n')
})

test('exceptions work', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `
      throw new Exception('Hello, from PHP!');
    `
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php'
  })

  const res = await php.handleRequest(req)

  // TODO: should exceptions be thrown back to the caller?
  t.assert(/Hello, from PHP!/.test(res.exception))
})

test('request headers work', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `
      $headers = apache_request_headers();
      echo $headers["X-Test"];
    `
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php',
    headers: {
      'X-Test': ['Hello, from Node.js!']
    }
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.body.toString(), 'Hello, from Node.js!')
})
