const { spawnSync } = require('node:child_process')
const { existsSync } = require('node:fs')
const { dirname, join } = require('node:path')

const debug = process.argv.includes('--debug')
const profile = debug ? 'debug' : 'release'
const targetDir = join(__dirname, 'target')

function run(command, args, env = process.env) {
  const result = spawnSync(command, args, { cwd: __dirname, stdio: 'inherit', env })
  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
}

// The CLI and Rust crate must use the same wasm-bindgen version.
const localBindgen = join(__dirname, '.tools', process.platform === 'win32' ? 'wasm-bindgen.exe' : 'wasm-bindgen')
const bindgen = process.env.WASM_BINDGEN || (existsSync(localBindgen) ? localBindgen : 'wasm-bindgen')
const version = spawnSync(bindgen, ['--version'], { encoding: 'utf8' })
if (version.error) throw version.error
if (version.status !== 0 || version.stdout.trim() !== 'wasm-bindgen 0.2.129') {
  throw new Error('Install wasm-bindgen-cli 0.2.129 to match Cargo.toml')
}

const check = process.argv.includes('--check')
const args = [check ? 'clippy' : 'build', '--target', 'wasm32-unknown-unknown', '--target-dir', targetDir]
if (!debug) args.push('--release')
if (check) args.push('--', '-D', 'warnings')
// Override the parent workspace's custom random backend for this JS-hosted WASM build.
const cargoEnv = {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: ['--cfg', 'getrandom_backend="wasm_js"', '-C', 'link-arg=-zstack-size=2097152'].join('\x1f'),
}
run('cargo', args, cargoEnv)
if (check) process.exit(0)
run(bindgen, [
  join(targetDir, 'wasm32-unknown-unknown', profile, 'node_engine.wasm'),
  '--target',
  'nodejs',
  '--out-dir',
  'wasm',
  '--out-name',
  'node_engine',
])
const typescriptPackage = require.resolve('typescript/package.json')
const tsc = join(dirname(typescriptPackage), require(typescriptPackage).bin.tsc)
run(process.execPath, [tsc, '-p', 'tsconfig.json'])
