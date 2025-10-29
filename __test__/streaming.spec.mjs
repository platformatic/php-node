import test from 'ava'

import { Php, Request } from '../index.js'
import { MockRoot } from './util.mjs'

test('handleStream - basic response', async (t) => {
  const mockroot = await MockRoot.from({
    'index.php': `<?php
      echo 'Hello, from PHP!';
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/index.php'
  })

  const [res] = await Promise.all([
    php.handleStream(req),
    req.end()
  ])

  t.is(res.status, 200)

  // Collect streaming body
  let body = ''
  for await (const chunk of res) {
    body += chunk.toString('utf8')
  }
  t.is(body, 'Hello, from PHP!')
})

test('handleStream - chunked output', async (t) => {
  const mockroot = await MockRoot.from({
    'stream.php': `<?php
      echo 'Chunk 1';
      flush();
      echo 'Chunk 2';
      flush();
      echo 'Chunk 3';
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/stream.php'
  })

  const [res] = await Promise.all([
    php.handleStream(req),
    req.end()
  ])

  t.is(res.status, 200)

  // Collect all chunks
  const chunks = []
  for await (const chunk of res) {
    chunks.push(chunk.toString('utf8'))
  }

  // Should have received all chunks
  const body = chunks.join('')
  t.is(body, 'Chunk 1Chunk 2Chunk 3')
})

test('handleStream - headers available immediately', async (t) => {
  const mockroot = await MockRoot.from({
    'headers.php': `<?php
      header('X-Custom-Header: test-value');
      header('Content-Type: application/json');
      echo '{"status": "ok"}';
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/headers.php'
  })

  const [res] = await Promise.all([
    php.handleStream(req),
    req.end()
  ])

  // Headers should be available immediately
  t.is(res.status, 200)
  t.is(res.headers.get('x-custom-header'), 'test-value')
  t.is(res.headers.get('content-type'), 'application/json')

  // Body can be consumed after
  let body = ''
  for await (const chunk of res) {
    body += chunk.toString('utf8')
  }
  t.is(body, '{"status": "ok"}')
})

test('handleStream - POST with buffered body', async (t) => {
  const mockroot = await MockRoot.from({
    'echo.php': `<?php
      $input = file_get_contents('php://input');
      echo "Received: " . $input;
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'POST',
    url: 'http://example.com/echo.php',
    headers: {
      'Content-Type': 'text/plain'
    },
    body: Buffer.from('Hello from client!')
  })

  const res = await php.handleStream(req)
  t.is(res.status, 200)

  let body = ''
  for await (const chunk of res) {
    body += chunk.toString('utf8')
  }
  t.is(body, 'Received: Hello from client!')
})

test('handleStream - POST with streamed body', async (t) => {
  const mockroot = await MockRoot.from({
    'echo.php': `<?php
      $input = file_get_contents('php://input');
      echo "Received: " . $input;
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'POST',
    url: 'http://example.com/echo.php',
    headers: {
      'Content-Type': 'text/plain'
    }
  })

  // Run handleStream and writes concurrently using Promise.all
  const [res] = await Promise.all([
    php.handleStream(req),
    (async () => {
      await req.write('Hello ')
      await req.write('from ')
      await req.write('streaming!')
      await req.end()
    })()
  ])

  t.is(res.status, 200)

  let body = ''
  for await (const chunk of res) {
    body += chunk.toString('utf8')
  }
  t.is(body, 'Received: Hello from streaming!')
})

test.skip('handleStream - exception handling', async (t) => {
  // TODO: Implement proper exception handling in streaming mode
  // See EXCEPTIONS.md for implementation approaches
  const mockroot = await MockRoot.from({
    'error.php': `<?php
      throw new Exception('Test exception');
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/error.php'
  })

  const res = await php.handleStream(req)

  // Exception should be sent through the stream
  let errorOccurred = false
  try {
    for await (const chunk of res) {
      // Should not receive chunks, should throw
    }
  } catch (err) {
    errorOccurred = true
    t.true(err.message.includes('Exception'))
  }

  t.true(errorOccurred, 'Exception should be thrown during iteration')
})

test('handleStream - empty response', async (t) => {
  const mockroot = await MockRoot.from({
    'empty.php': `<?php
      // No output
    ?>`
  })
  t.teardown(() => mockroot.clean())

  const php = new Php({
    docroot: mockroot.path
  })

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/empty.php'
  })

  const [res] = await Promise.all([
    php.handleStream(req),
    req.end()
  ])

  t.is(res.status, 200)

  let body = ''
  for await (const chunk of res) {
    body += chunk.toString('utf8')
  }
  t.is(body, '')
})
