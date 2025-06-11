import { randomUUID } from 'node:crypto'
import { writeFile, mkdir, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const base = tmpdir()

export class MockRoot {
  /**
   * Creates a mock docroot using a nested object to represent the directory
   * structure. A directory has a
   *
   * Example:
   *
   * ```js
   * const dir = mockroot({
   *   'hello.txt': 'Hello, World!',
   *   'subdir': {
   *     'subfile.txt': Buffer.from('hi')
   *   }
   * })
   * ```
   *
   * @param {*} files
   */
  constructor(name = randomUUID()) {
    this.path = join(base, name)
  }

  async writeFiles(files, base = this.path) {
    await mkdir(base, { recursive: true })

    for (let [name, contents] of Object.entries(files)) {
      if (typeof contents === 'string') {
        contents = Buffer.from(contents)
      }

      const path = join(base, name)
      if (Buffer.isBuffer(contents)) {
        await writeFile(path, contents)
      } else {
        await this.writeFiles(contents, path)
      }
    }
  }

  static async from(files) {
    const mockroot = new MockRoot()
    await mockroot.writeFiles(files)
    return mockroot
  }

  /**
   * Cleanup the mock docroot
   */
  async clean() {
    await rm(this.path, {
      recursive: true,
      force: true
    })
  }
}
