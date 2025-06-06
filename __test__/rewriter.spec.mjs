import test from 'ava'

import { Request, Rewriter } from '../index.js'

const docroot = import.meta.dirname

test('existence condition', (t) => {
  const req = new Request({
    method: 'GET',
    url: 'http://example.com/util.mjs',
    headers: {
      TEST: ['foo']
    }
  })

  const rewriter = new Rewriter([
    {
      conditions: [
        { type: 'exists' }
      ],
      rewriters: [
        {
          type: 'path',
          args: ['.*', '/404']
        }
      ]
    }
  ])

  t.is(rewriter.rewrite(req, docroot).url, 'http://example.com/404')
})

test('non-existence condition', (t) => {
  const req = new Request({
    method: 'GET',
    url: 'http://example.com/index.php',
    headers: {
      TEST: ['foo']
    }
  })

  const rewriter = new Rewriter([
    {
      conditions: [
        { type: 'not_exists' }
      ],
      rewriters: [
        {
          type: 'path',
          args: ['.*', '/404']
        }
      ]
    }
  ])

  t.is(rewriter.rewrite(req, docroot).url, 'http://example.com/404')
})

test('condition groups - AND', (t) => {
  const rewriter = new Rewriter([{
    conditions: [
      { type: 'header', args: ['TEST', 'foo'] },
      { type: 'path', args: ['^(/index.php)$'] }
    ],
    rewriters: [
      { type: 'path', args: ['^(/index.php)$', '/foo$1'] }
    ]
  }])

  // Both conditions match, so rewrite is applied
  {
    const req = new Request({
      method: 'GET',
      url: 'http://example.com/index.php',
      headers: {
        TEST: ['foo']
      }
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/foo/index.php'
    )
  }

  // Header condition does not match, so rewrite is not applied
  {
    const req = new Request({
      method: 'GET',
      url: 'http://example.com/index.php'
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/index.php'
    )
  }

  // Path condition does not match, so rewrite is not applied
  {
    const req = new Request({
      method: 'GET',
      url: 'http://example.com/nope.php',
      headers: {
        TEST: ['foo']
      }
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/nope.php'
    )
  }
})

test('condition groups - OR', (t) => {
  const rewriter = new Rewriter([{
    operation: 'or',
    conditions: [
      { type: 'method', args: ['GET'] },
      { type: 'path', args: ['^(/index.php)$'] }
    ],
    rewriters: [
      { type: 'path', args: ['^(.*)$', '/foo$1'] }
    ]
  }])

  // Both conditions match, so rewrite is applied
  {
    const req = new Request({
      url: 'http://example.com/index.php'
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/foo/index.php'
    )
  }

  // Path condition matches, so rewrite is applied
  {
    const req = new Request({
      url: 'http://example.com/index.php'
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/foo/index.php'
    )
  }

  // Header condition matches, so rewrite is applied
  {
    const req = new Request({
      url: 'http://example.com/nope.php'
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/foo/nope.php'
    )
  }

  // Neither condition matches, so rewrite is not applied
  {
    const req = new Request({
      method: 'POST',
      url: 'http://example.com/nope.php'
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/nope.php'
    )
  }
})

test('header rewriting', (t) => {
  const rewriter = new Rewriter([{
    rewriters: [
      { type: 'header', args: ['TEST', '(.*)', '${1}bar'] }
    ]
  }])

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/index.php',
    headers: {
      TEST: ['foo']
    }
  })

  t.is(rewriter.rewrite(req, docroot).headers.get('TEST'), 'foobar')
})

test('href rewriting', (t) => {
  const rewriter = new Rewriter([{
    rewriters: [
      { type: 'href', args: [ '^(.*)$', '/index.php?route=${1}' ] }
    ]
  }])

  const req = new Request({
    url: 'http://example.com/foo/bar'
  })

  t.is(
    rewriter.rewrite(req, docroot).url,
    'http://example.com/index.php?route=/foo/bar'
  )
})

test('method rewriting', (t) => {
  const rewriter = new Rewriter([{
    rewriters: [
      { type: 'method', args: ['GET', 'POST'] }
    ]
  }])

  const req = new Request({
    url: 'http://example.com/index.php'
  })

  t.is(rewriter.rewrite(req, docroot).method, 'POST')
})

test('path rewriting', (t) => {
  const rewriter = new Rewriter([{
    rewriters: [
      { type: 'path', args: ['^(/index.php)$', '/foo$1'] }
    ]
  }])

  const req = new Request({
    method: 'GET',
    url: 'http://example.com/index.php',
    headers: {
      TEST: ['foo']
    }
  })

  t.is(rewriter.rewrite(req, docroot).url, 'http://example.com/foo/index.php')
})

test('rewriter sequencing', (t) => {
  const rewriter = new Rewriter([{
    conditions: [
      { type: 'path', args: ['^(/index.php)$'] }
    ],
    rewriters: [
      { type: 'path', args: ['^(/index.php)$', '/bar$1'] },
      { type: 'path', args: ['^(/bar)', '/foo$1'] }
    ]
  }])

  // Condition matches, and both rewriters are applied in sequence
  {
    const req = new Request({
      method: 'GET',
      url: 'http://example.com/index.php',
      headers: {
        TEST: ['foo']
      }
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/foo/bar/index.php'
    )
  }

  // Condition does not match, so no rewrites are applied even if the second
  // rewriter would match
  {
    const req = new Request({
      method: 'GET',
      url: 'http://example.com/bar/baz.php',
      headers: {
        TEST: ['foo']
      }
    })

    t.is(
      rewriter.rewrite(req, docroot).url,
      'http://example.com/bar/baz.php'
    )
  }
})
