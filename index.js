const {
  Php,
  Headers,
  Request,
  Response
} = getNativeBinding(process)

module.exports = {
  Php,
  Headers,
  Request,
  Response
}

function isMusl() {
  const { header } = process.report.getReport()
  return typeof header.glibcVersionRuntime === 'undefined'
}

function getNativeBinding({ platform, arch }) {
  let name = `${platform}-${arch}`
  if (platform === 'linux') {
    name += isMusl() ? '-musl' : '-gnu'
    if (arch === 'arm') name += 'abihf'
  } else if (platform === 'win32') {
    name += '-msvc'
  }

  const path = process.env.TEST
    ? `./php.${name}.node`
    : `./npm/${name}/binding.node`

  return require(path)
}
