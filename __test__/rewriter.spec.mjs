import test from 'ava'

import { Request, Rewriter } from '../index.js'

test('rewrites URLs', (t) => {
  const req = new Request({
    method: 'GET',
    url: 'http://example.com/index.php',
    headers: {
      TEST: ['foo']
    }
  })

  const rewriter = new Rewriter([
    {
      operation: 'and',
      conditions: [
        {
          type: 'path',
          args: ['^/index.php$']
        },
        {
          type: 'header',
          args: ['TEST', '^foo$']
        }
      ],
      rewriters: [
        {
          type: 'path',
          args: ['^(/index.php)$', '/foo$1']
        }
      ]
    }
  ])

  t.is(rewriter.rewrite(req).url, 'http://example.com/foo/index.php')
})
