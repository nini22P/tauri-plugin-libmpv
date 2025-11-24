import { readFileSync } from 'fs'
import { join, dirname } from 'path'
import { cwd } from 'process'
import typescript from '@rollup/plugin-typescript'

const pkg = JSON.parse(readFileSync(join(cwd(), 'package.json'), 'utf8'))

const outputDir = dirname(pkg.exports.import)

export default [
  {
    input: 'guest-js/index.ts',
    output: [
      {
        file: pkg.exports.import,
        format: 'esm'
      },
      {
        file: pkg.exports.require,
        format: 'cjs'
      }
    ],
    plugins: [
      typescript({
        declaration: true,
        declarationDir: outputDir,
      })
    ],
    external: [
      /^@tauri-apps\/api/,
      ...Object.keys(pkg.dependencies || {}),
      ...Object.keys(pkg.peerDependencies || {})
    ]
  },
  {
    input: 'guest-js/cli.ts',
    output: {
      file: './dist-js/cli.cjs',
      format: 'cjs',
      banner: '#!/usr/bin/env node'
    },
    plugins: [
      typescript({
        declaration: false,
      })
    ],
    external: [
      'fs', 'path', 'os', 'child_process', 'stream', 'stream/promises', 'https', 'http', 'url', 'util', '7z-wasm',
      ...Object.keys(pkg.dependencies || {}),
    ]
  }
]