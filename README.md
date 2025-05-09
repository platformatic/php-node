# @platformatic/php

Delegate handling of HTTP requests to a thread pool of PHP instances.

## Requirements

PHP dynamically links against several system libraries. These must be installed
as listed below:

### Linux

```sh
sudo apt-get update
sudo apt-get install -y libssl-dev libcurl4-openssl-dev libxml2-dev \
  libsqlite3-dev libonig-dev re2c
```

### macOS

```sh
brew install openssl@3 curl sqlite libxml2 oniguruma
```

## Install

```sh
npm install @platformatic/php
```

## Usage

```js
import { Php, Request } from '@platformatic/php'

// Construct a PHP environment for handling requests.
// This corresponds to a single entrypoint file.
// Presently the file contents must be passed in as a string,
// but it could be made to take only a filename and read the file
// contents itself.
const php = new Php({
  file: 'index.php',
  code: `<?php
    $headers = apache_request_headers();
    echo $headers["X-Test"];
  ?>`
})

// This is a container to help translate Node.js requests into PHP requests.
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
