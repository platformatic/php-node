import { join, resolve } from 'path'
import { readFileSync } from 'fs'
import { createServer } from 'http'
import { Php, Request } from '../index.js'
import { strictEqual } from 'assert'

// Read homepage html
const { dirname } = import.meta
const homepage = readFileSync(join(dirname, 'index.html'), 'utf8')

// Create reusable PHP instance
const php = new Php({
  file: 'index.php',
  code: readFileSync(resolve(dirname, 'index.php'), 'utf8')
})

const server = createServer(async (req, res) => {
  // TODO: We need to buffer the whole request rather than streaming to PHP.
  // Need to add streaming support to lang_handler and the php crate.
  const chunks = []
  for await (const chunk of req) {
    chunks.push(chunk)
  }

  const url = urlForRequest(req)

  // Every page except /info should show the homepage.
  if (url.pathname !== '/info') {
    res.writeHead(200, {
      'Content-Type': 'text/html'
    })
    res.end(homepage)
    return
  }

  const request = new Request({
    method: req.method,
    url: url.href,
    headers: fixHeaders(req.headers),
    body: Buffer.concat(chunks)
  })

  try {
    const response = await php.handleRequest(request)
    res.writeHead(response.status, response.headers)
    res.end(response.body)
  } catch (err) {
    res.writeHead(500, {
      'Content-Type': 'text/plain'
    })
    res.end(err.message)
  }
})

server.listen(3000, async () => {
  const { port } = server.address()
  const url = `http://localhost:${port}/info`

  const res = await fetch(url, {
    method: 'POST',
    body: 'Hello, from Node.js!'
  })

  const response = await res.text()
  strictEqual(response, 'Hello, from PHP!')
  console.log(response)
  console.log()

  console.log(`Try a request to http://localhost:${port}/ to see the phpinfo() output.`)
})

//
// Helpers
//

// A full URL string is needed for PHP, but Node.js splits that across a bunch of places.
function urlForRequest(req) {
  const proto = req.protocol ?? 'http:'
  const host = req.headers.host ?? 'localhost'
  return new URL(req.url, `${proto}//${host}`)
}

// Currently header values must be arrays. Need to make it support single values too.
function fixHeaders(headers) {
  return Object.fromEntries(
    Object.entries(headers)
      .map(([key, value]) => [key, [value]])
  )
}
