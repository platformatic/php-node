const {
  Php,
  Headers,
  Request,
  Response,
  Rewriter
} = getNativeBinding(process)

module.exports = {
  Php,
  Headers,
  Request,
  Response,
  Rewriter
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

  try {
    return require(`./npm/${name}/binding.node`)
  } catch (err) {
    if (err.code === 'MODULE_NOT_FOUND') {
      return require(`./php.${name}.node`)
    }
    throw err
  }
}
