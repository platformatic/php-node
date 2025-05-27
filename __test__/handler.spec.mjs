import test from 'ava'

import { Php, Request } from '../index.js'

import { MockRoot } from './util.mjs'

test('Support input/output streams', async (t) => {
  const mockroot = await MockRoot.from({
    'index.php': `<?php
      if (file_get_contents('php://input') == 'Hello, from Node.js!') {
        echo 'Hello, from PHP!';
      }
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'POST',
    url: 'http://example.com/index.php',
    body: Buffer.from('Hello, from Node.js!')
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.body.toString('utf8'), 'Hello, from PHP!')
})

test('Capture logs', async (t) => {
  const mockroot = await MockRoot.from({
    'index.php': `<?php
      error_log('Hello, from error_log!');
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    url: 'http://example.com/index.php'
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.log.toString('utf8'), 'Hello, from error_log!\n')
})

test('Capture exceptions', async (t) => {
  const mockroot = await MockRoot.from({
    'index.php': `<?php
      throw new Exception('Hello, from PHP!');
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    url: 'http://example.com/index.php'
  })

  const res = await php.handleRequest(req)

  // TODO: should exceptions be thrown rather than message-captured?
  t.assert(/Hello, from PHP!/.test(res.exception))
})

test('Support request and response headers', async (t) => {
  const mockroot = await MockRoot.from({
    'index.php': `<?php
      $headers = apache_request_headers();
      header("X-Test: Hello, from PHP!");
      // TODO: Does PHP expect headers be returned to uppercase?
      echo $headers["X-Test"];
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    url: 'http://example.com/index.php',
    headers: {
      'X-Test': ['Hello, from Node.js!']
    }
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)
  t.is(res.body.toString(), 'Hello, from Node.js!')
  t.is(res.headers.get('X-Test'), 'Hello, from PHP!')
})

test('Has expected args', async (t) => {
  const mockroot = await MockRoot.from({
    'index.php': `<?php
      echo "[";
      $first = true;
      foreach ($argv as $value) {
        if ($first) { $first = false; }
        else { echo ","; }
        echo "\\"$value\\"";
      }
      echo "]";
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    argv: process.argv,
    docroot: mockroot.path
  })

  const req = new Request({
    url: 'http://example.com/index.php'
  })

  const res = await php.handleRequest(req)
  t.is(res.status, 200)

  t.is(res.body.toString('utf8'), JSON.stringify(process.argv))
})
