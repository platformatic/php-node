# @platformatic/php-node

With `@platformatic/php-node` you can run PHP applications within the same
process as a Node.js application, allowing for communication between Node.js
and PHP without any network connection in the middle. This allows for some
interesting possibilities, like running Wordpress with a Next.js frontend.

## Requirements

Presently support is provided for x64 Linux and both x64 and arm64 macOS. More
platforms will come as needs arise. Please open an issue if we're missing a
platform you would like supported!

PHP dynamically links against several system libraries. These must be installed
as listed below:

### Linux

```sh
sudo apt-get update
sudo apt-get install -y libssl-dev libcurl4-openssl-dev libxml2-dev \
  libsqlite3-dev libonig-dev re2c libpq5
```

### macOS

```sh
brew install openssl@3 curl sqlite libxml2 oniguruma postgresql@16
```

## Install

```sh
npm install @platformatic/php-node
```

## Usage

```js
import { Php, Request } from '@platformatic/php-node'

const php = new Php()

const request = new Request({
  url: 'http://example.com/foo/bar',
  headers: {
    'X-Test': ['Hello, from Node.js!']
  }
})

const response = await php.handleRequest(request)

console.log(response.body.toString())
```

## API

### `new Php(config)`

* `config` {Object} Configuration object
  * `argv` {String[]} Process arguments. **Default:** []
  * `docroot` {String} Document root for PHP. **Default:** process.cwd()
  * `throwRequestErrors` {Boolean} Throw request errors rather than returning
    responses with error codes. **Default:** false
  * `rewriter` {Rewriter} Optional rewrite rules. **Default:** `undefined`
* Returns: {Php}

Construct a new PHP instance to which to dispatch requests.

```js
import { Php } from '@platformatic/php-node'

const php = new Php({
  argv: process.argv,
  docroot: process.cwd()
})
````

### `php.handleRequest(request)`

* `request` {Request} A request to dispatch to the PHP instance.
* Returns: {Promise<Response>}

When the request completes, the returned promise will resolve with the response
object. Request processing is handled by the NodePlatform worker pool to avoid
blocking the Node.js thread.

```js
import { Php, Request } from '@platformatic/php-node'

const php = new Php()
const request = new Request({
  url: 'http://example.com/foo/bar'
})

const response = await php.handleRequest(request)
console.log(response.body.toString())
````

### `php.handleRequestSync(request)`

* `request` {Request} A request to dispatch to the PHP instance.
* Returns: {Response}

Requests may also be processed synchronously, though this is not recommended as
it will block the Node.js thread for the entire life of the PHP request.

This may be useful for one-off scripts. It's only included because it's trivial
to do so, but it's not recommended for use within HTTP requests.

```js
import { Php, Request } from '@platformatic/php-node'

const php = new Php()
const request = new Request({
  url: 'http://example.com/foo/bar'
})

const response = php.handleRequestSync(request)
console.log(response.body.toString())
```

### `new Request(input)`

* `input`
  * `method` {String} HTTP method **Default:** `GET`
  * `url` {String} Full request URL
  * `headers` {Object} HTTP request headers. Each must be an array of strings
  * `body` {Buffer|UInt8Array} Request body
* Returns: {Request}

Construct a request which may be dispatched to a PHP instance.

```js
import { Request } from '@platformatic/php-node'

const request = new Request({
  method: 'POST',
  url: 'http://example.com/foo/bar',
  headers: {
    'Content-Type': ['application/json']
  },
  body: Buffer.from(JSON.stringify({
    hello: 'world'
  }))
})
```

### `request.method`

* {String}

The HTTP method to use when dispatching this request.

```js
import { Request } from '@platformatic/php-node'

const request = new Request({
  url: 'http://example.com/foo/bar',
})

console.log(request.method) // GET
```

### `request.url`

* {String}

The URL to use when dispatching this request.

```js
import { Request } from '@platformatic/php-node'

const request = new Request({
  url: 'http://example.com/foo/bar',
})

console.log(request.url) // http://example.com/foo/bar
```

### `request.headers`

* {Headers}

The HTTP headers to use when dispatching this request.

```js
import { Request } from '@platformatic/php-node'

const request = new Request({
  url: 'http://example.com/foo/bar',
})

console.log(request.headers) // [Headers]
```

### `request.body`

* {Buffer}

The body to use when dispatching this request.

```js
import { Request } from '@platformatic/php-node'

const request = new Request({
  url: 'http://example.com/foo/bar',
  body: Buffer.from('Hello, world!')
})

console.log(request.body.toString()) // Hello, world!
```

### `new Response(input)`

* `input` {Object} Response values.
  * `status` {Number} HTTP Response status code
  * `headers` {Object} HTTP Response headers. Each must be an array of strings
  * `body` {Buffer} HTTP Response body
  * `log` {String} Log output of this request
* Returns: {Response}

Responses may be constructed manually. This is mainly just for testing, but may
have other uses, like short-circuiting the PHP instance run entirely in certain
cases.

```js
import { Response } from '@platformatic/php-node'

const response = new Response({
  status: 500,
  headers: {
    'Content-Type': ['application/json']
  },
  body: Buffer.from(JSON.stringify({
    error: 'bad stuff'
  }))
})
```

### `response.status`

* {Number}

The HTTP status code included in the response.

```js
import { Response } from '@platformatic/php-node'

const response = new Response({
  status: 500
})

console.log(response.status) // 500
```

### `response.headers`

* {Headers}

The HTTP headers included in the response.

```js
import { Response } from '@platformatic/php-node'

const response = new Response({
  headers: {
    'Content-Type': ['application/json']
  },
})

console.log(response.headers) // [Headers]
```

### `response.body`

* {Buffer}

The HTTP response body.

```js
import { Response } from '@platformatic/php-node'

const response = new Response({
  body: Buffer.from(JSON.stringify({
    error: 'bad stuff'
  }))
})

console.log(response.body.toString()) // {"error":"bad stuff"}
```

### `response.log`

* {Buffer}

Any logs captured during the request.

```js
import { Response } from '@platformatic/php-node'

const response = new Response({
  log: Buffer.from('some log message')
})

console.log(response.log.toString()) // some log message
```

### `new Headers()`

* Returns: {Headers}

Construct a Headers object to manage HTTP headers. Note that this is currently
only useful for reading _from_ Request and Response types, not passing _into_
them.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()
```

### `headers.set(name, value)`

* `name` {String} The header name for which to set a value.
* `value` {String} The value to set for the named header.

This will set the value of the named header. If any prior headers have been
set with this name they will be discarded.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
```

### `headers.add(name, value)`

* `name` {String} The header name for which to add a value.
* `value` {String} The value to add for the named header.

This will add to the associated values of the named header. If any prior
headers have been set with this name they will also be kept.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')
```

### `headers.has(name)`

* Returns: {bool}

Checks if there are any values currently associated with the header name.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.has('Content-Type') // false
```

### `headers.get(name)`

* Returns: {string|undefined}

Retrieves the last value associated with the given header name.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

headers.get('Accept') // text/html
```

### `headers.getAll(name)`

* Returns: {String[]}

Retrieves all values associated with the given header name.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

headers.getAll('Accept') // ['application/json', 'text/html']
```

### `headers.getLine(name)`

* Returns: {String|undefined}

Merges all associated values into one header line. Note that his may be
incorrect for some header types which require separate header lines such as
the `Set-Cookie` header.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

headers.getLine('Accept') // application/json, text/html
```

### `headers.delete(name)`

Delete all values associated with the given header name.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')
headers.delete('Accept')

headers.get('Accept') // undefined
```

### `headers.clear()`

Remove all contained headers.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
headers.add('Accept', 'application/json')
headers.clear()

headers.get('Content-Type') // undefined
headers.get('Accept') // undefined
```

### `headers.size`

* {Number}

The number of header names present.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

headers.size // 3
```

### `headers.entries()`

* {Iterator}

Returns an iterator containing a `(name, value)` tuple of header entries.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

for (const (name, value) of headers.entries()) {
  // ('Content-Type', 'application/json')
  // ('Accept', 'application/json')
  // ('Accept', 'text/html')
}
```

### `headers.keys()`

* {Iterator}

Returns an iterator of header names.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

for (const name of headers.keys()) {
  // 'Content-Type'
  // 'Accept'
}
```

### `headers.values()`

* {Iterator}

Returns an iterator of header values.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

for (const value of headers.values()) {
  // 'application/json'
  // 'application/json'
  // 'text/html'
}
```

### `headers.forEach(fn)`

* `fn` {Function} Callback to call for each header entry
  * `value` {String} The value of the header entry.
  * `name` {String} The name of the header entry.
  * `headers` {Headers} The Header instance

Iterate over each header entry with a given callback.

```js
import { Headers } from '@platformatic/php-node'

const headers = new Headers()

headers.set('Content-Type', 'application/json')
headers.add('Accept', 'application/json')
headers.add('Accept', 'text/html')

headers.forEach((value, name, headers) => {
  // ('application/json', 'Content-Type', headers)
  // ('application/json', 'Accept', headers)
  // ('text/html', 'Accept', headers)
})
```

### `new Rewriter(input)`

* `rules` {Array} The set of rewriting rules to apply to each request
  * `operation` {String} Operation type (`and` or `or`) **Default:** `and`
  * `conditions` {Array} Conditions to match against the request
    * `type` {String} Condition type
    * `args` {String} Arguments to pass to condition constructor
  * `rewriters` {Array} Rewrites to apply if the conditions match
    * `type` {String} Rewriter type
    * `args` {String} Arguments to pass to rewriter constructor
* Returns: {Rewriter}

Construct a Rewriter to rewrite requests before they are dispatched to PHP.

```js
import { Rewriter } from '@platformatic/php-node'

const rewriter = new Rewriter([{
  conditions: [{
    type: 'header',
    args: ['User-Agent', '^(Mozilla|Chrome)']
  }],
  rewriters: [{
    type: 'path',
    args: ['^/old-path/(.*)$', '/new-path/$1']
  }]
}])
```

#### Conditions

There are several types of conditions which may be used to match against the
request. Each condition type has a set of arguments which are passed to the
constructor of the condition. The condition will be evaluated against the
request and if it matches, the rewriters will be applied.

The available condition types are:

- `exists` Matches if request path exists in docroot.
- `not_exists` Matches if request path does not exist in docroot.
- `header(name, pattern)` Matches named header against a pattern.
  - `name` {String} The name of the header to match.
  - `pattern` {String} The regex pattern to match against the header value.
- `method(pattern)` Matches request method against a pattern.
  - `pattern` {String} The regex pattern to match against the HTTP method.
- `path(pattern)`: Matches request path against a pattern.
  - `pattern` {String} The regex pattern to match against the request path.

#### Rewriters

There are several types of rewriters which may be used to rewrite the request
before it is dispatched to PHP. Each rewriter type has a set of arguments which
are passed to the constructor of the rewriter. The rewriter will be applied to
the request if the conditions match.

The available rewriter types are:

- `header(name, replacement)` Sets a named header to a given replacement.
  - `name` {String} The name of the header to set.
  - `replacement` {String} The replacement string to use for named header.
- `href(pattern, replacement)` Rewrites request path, query, and fragment to
  given replacement.
  - `pattern` {String} The regex pattern to match against the request path.
  - `replacement` {String} The replacement string to use for request path.
- `method(replacement)` Sets the request method to a given replacement.
  - `replacement` {String} The replacement string to use for request method.
- `path(pattern, replacement)` Rewrites request path to given replacement.
  - `pattern` {String} The regex pattern to match against the request path.
  - `replacement` {String} The replacement string to use for request path.

### `rewriter.rewrite(request, docroot)`

- `request` {Object} The request object.
- `docroot` {String} The document root.

Rewrites the given request using the rules provided to the rewriter.

This is mainly exposed for testing purposes. It is not recommended to use
directly. Rather, the `rewriter` should be provided to the `Php` constructor
to allow rewriting to occur within the PHP environment where it will be aware
of the original `REQUEST_URI` state.

```js
import { Rewriter } from '@platformatic/php-node'

const rewriter = new Rewriter([{
  rewriters: [{
    type: 'path',
    args: ['^(.*)$', '/base/$1'
  }]
}])

const request = new Request({
  url: 'http://example.com/foo/bar'
})

const modified = rewriter.rewrite(request, import.meta.dirname)

console.log(modified.url) // http://example.com/base/foo/bar
```

## Contributing

This project is part of the [Platformatic](https://github.com/platformatic)
ecosystem. Please refer to the main repository for contribution guidelines.

## License

Apache-2.0

## Support

- [GitHub Issues](https://github.com/platformatic/php-node/issues)
- [Platformatic Documentation](https://docs.platformatic.dev/)
- [Community Discord](https://discord.gg/platformatic)
