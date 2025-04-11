# php-stackable

Proof-of-concept PHP stackable. Not yet working...

## Build Notes

### Building PHP

Building PHP itself is straightforward. Here's the basic configuration:

```sh
git clone https://github.com/php/php-src.git
cd php-src
./buildconf
./configure --enable-shared --enable-embed=shared --enable-zts --without-iconv --with-pdo-mysql=mysqlnd --with-mysqli=mysqlnd --with-openssl --with-curl --enable-mbstring
make -j$([[ "$(uname)" == "Darwin" ]] && sysctl -n hw.physicalcpu || nproc)
sudo make install
```

We'll probably want to build with additional extensions later, but this is a
good starting point. Extensions should be able to load dynamically anyway,
so easy enough to add them separately.

### ext-php-rs

This presently expects the [Complete SAPI Implementation PR](https://github.com/platformatic/ext-php-rs/pull/1)
to be checked out at the same level as this repo under the name ext-php-rs.

```
base
├── ext-php-rs
└── php-stackable
```

### Building the Node.js module

Presently the rpath to link needs to be configured via environment variable.
This tells the linker where to find the PHP shared library.

```sh
RUSTFLAGS="-C link-args=-Wl,-rpath,/usr/local/lib" npm run build
```

By default rpath is left to its default, which I _think_ means cwd, but I need
to verify that. It can be configured in `build.rs` to a different location,
but that is likely platform-specific so would need to figure out the correct
locations if we want to use platform-available PHP builds.

Alternatively, we could ship our own libphp alongside the .node file, but I
need to figure out how to configure the rpath correctly to work with the
relative path. This may be the better option, but would also need to figure
out if that then dictates where extensions need to live.

## Usage

```js
import { Php, Request } from '@platformatic/php'

// Construct a PHP environment for handling requests.
// This corresponds to a single entrypoint file.
// Presently the file contents must be passed in as a string,
// but it could be made to take only a filename and read the file
// contents itself.
//
// NOTE: This presently only supports eval-mode, not tag-mode, meaning no
// interleaving with html using <?php ?> tags. Tag mode will be ready soon.
const php = new Php({
  file: 'index.php',
  code: `
    $headers = apache_request_headers();
    echo $headers["X-Test"];
  `
})

// This is a container to help translate Node.js requests into PHP requests.
//
// Future ideas:
// - Support passing in a Node.js IncomingMessage object directly?
// - Support web standard Request objects?
const req = new Request({
  method: 'GET',
  url: 'http://example.com/test.php',
  headers: {
    'X-Test': ['Hello, from Node.js!']
  }
})

// The request container gets passed into the PHP environment which processes
// it and returns a response. Request processing is handled by the NodePlatform
// worker pool to avoid blocking the Node.js thread.
const res = await php.handleRequest(req)

// PHP requests can also be processed synchronously, though this is not
// recommended as it will block the Node.js thread for the entire life of the
// PHP request. It may be useful in some cases though.
const res = php.handleRequestSync(req)

// Properties available on Response objects:
console.log({
  status: res.status, // status code
  headers: new Map(res.headers.entries()), // headers is a Headers object
  body: res.body.toString(), // body is a Buffer
  log: res.log.toString(), // log is a Buffer
  exception: res.exception // exception is a string
})

// Headers is a multimap which implements all the standard Map methods plus
// some additional helpers. See the tests in __test__ for more details.
```

## Various learnings

### php://input

PHP has no concept of a "socket", it instead has its own form of streams which
can be mounted into a request run. The `php://input` stream represents the body
of an incoming request.

### php://output

As with `php://input`, `php://output` is a stream that can be mounted into a
request run, but is instead used for writing out to the response.

### superglobals

As PHP uses its input and output streams for transmitting _only_ the request
and response bodies, headers must be passed in separately. The way this is done
from the perspective of PHP is via what it calls "superglobals". These are
special variables which are global to every script.

The main superglobals of interest are:
 - `$_SERVER` contains information about the server and the request.
 - `$_GET` contains query string parameters.
 - `$_POST` contains form data.
 - `$_FILES` contains file uploads.
 - `$_COOKIE` contains cookies.
 - `$_SESSION` contains session data.
 - `$_REQUEST` is a mix of `$_GET`, `$_POST`, and `$_COOKIE`.
 - `$_ENV` contains environment variables.

Super globals are set from C prior to initiating the request using the
`SG(...)` macro. For example, `SG(request_info).request_method` is set to the
request method. The names given to `SG(...)` are poorly matched to the names of
the superglobals they are assigned to, so it is necessary to look at the
`php_variables.h` file to determine the correct name.

### SAPI -- The "recommended" embedding API

PHP has a concept of a "Server API" (SAPI) which is the interface between PHP
and the web server. The SAPI is responsible for handling the request and
response, and is the recommended way to embed PHP into a C application.

It is a simplification of the CGI interface, but is _too_ simplified to be
useful for our purposes. When used directly, it spins up an entirely fresh
instance of PHP for each request, which is suffers from a lot of startup cost,
and doesn't allow sharing code compilation between requests.

### Using the Zend API directly

All that SAPI actually does _internally_ is squash three (possibly four?)
nested scopes into one, but these are more useful to us separated.

#### (Optional) php_tsrm_startup (Thread Safe Resource Management)

Provides thread safety for running multiple PHP environments in parallel.

#### zend_signal_startup (Signal Handling)

Defines globally how PHP should handle signals, not configurable with SAPI.

#### sapi_startup (Server API)

Initializes the SAPI, and provides a way to configure it. This is really just
a container for loading INI settings, extensions, and allocating space for
superglobals on the current thread.

#### php_embed_module.startup

This is the only actually _configurable_ part of SAPI. It treats the
PHP server you're trying to construct as just another a module/extension,
which is a bit odd as the thing that is supposed to be orchestrating
everything.

Configuration of this stages is done through [one-big-struct](https://github.com/php/php-src/blob/6024122e54f4e8a4f35c0abe9b46425856a11e6c/main/SAPI.h#L237-L290)
which contains individual functions for:

  - reading POST data to populate `$_POST`
  - reading GET data to populate `$_GET`
  - reading cookies to populate `$_COOKIE`
  - reading environment variables to populate `$_ENV`
  - reading request headers to populate `$_SERVER`
  - reading request body to populate `php://input`
  - writing response headers
  - writing response body from `php://output`
  - Handling errors

#### php_request_startup (Request Startup)

This is the scope in which the actual request can occur. It allocates space
for the request-related superglobals, and sets up the request environment.
Within this scope PHP code can then be run with those request-specific
superglobals populated.

Within SAPI this stage is bundled into the startup of the entire SAPI system,
and so a SAPI construction can only handle a single request before tearing down
everything completely.

The _better_ way is to reuse this stage and the probably construct a separate
php_embed_module also for each request. In this way most of the PHP environment
can be shared between requests, and only the request-specific data needs to be
updated.

### Maybe PHP can also be concurrent?

PHP is designed to allow an environment to be shared across multiple threads
with the `tsrm` system. But as input and output are _streams_ it may also be
possible to run multiple requests on the same thread concurrently, to some
extent, by switching out their superglobal states whenever stream data would
be read, or when writing out would block the current request.

A caveat here is that _other_ than the input and output streams, things are
generally synchronous. For example, typical database drivers would block the
thread. Being _partially_ async may still be an improvement though, and there's
always the possibility of us writing our own async components, which would get
us better performance while also possibly locking in our users a bit more.
