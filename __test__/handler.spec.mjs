import test from 'ava'

import { Php, Request } from '../index.js'

test('Support input/output streams', async (t) => {
  const php = new Php({
    argv: process.argv,
    file: 'index.php',
    code: `<?php
      if (file_get_contents('php://input') == 'Hello, from Node.js!') {
        echo 'Hello, from PHP!';
      }
    ?>`
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

test('Capture logs', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `<?php
      error_log('Hello, from error_log!');
    ?>`
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php'
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.log.toString('utf8'), 'Hello, from error_log!\n')
})

test('Capture exceptions', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `<?php
      throw new Exception('Hello, from PHP!');
    ?>`
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/test.php'
  })

  const res = await php.handleRequest(req)

  // TODO: should exceptions be thrown rather than message-captured?
  t.assert(/Hello, from PHP!/.test(res.exception))
})

test('Support request and response headers', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `<?php
      $headers = apache_request_headers();
      header("X-Test: Hello, from PHP!");
      // TODO: Does PHP expect headers be returned to uppercase?
      echo $headers["x-test"];
    ?>`
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
  t.is(res.headers.get('X-Test'), 'Hello, from PHP!')
})
