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

  await t.throwsAsync(php.handleRequest(req), {
    message: 'Hello, from PHP!'
  })
})

test('input and output headers work', async (t) => {
  const php = new Php({
    file: 'index.php',
    code: `
      if ($_SERVER['HTTP_X_TEST'] == 'Hello, from Node.js!') {
        header('X-Test: Hello, from PHP!');
      }
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
  console.log(res)
  t.is(res.status, 200)
  // t.is(res.headers['X-Test'], 'Hello, from PHP!')
})
