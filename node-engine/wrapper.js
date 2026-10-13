const { NodeEngine } = require('./wasm/node_engine.js')

class Wrapper {
  constructor() {
    this.nodeEngine = new NodeEngine()
  }

  destroy() {
    if (!this.nodeEngine) return
    this.nodeEngine.destroy()
    this.nodeEngine.free()
    this.nodeEngine = null
  }

  takeState(rawState) {
    this.assertInitialized()
    return this.nodeEngine.takeState(rawState)
  }

  isEnabled(featureName, context) {
    this.assertInitialized()
    return this.nodeEngine.isEnabled(JSON.stringify({ ...context, toggleName: featureName }))
  }

  assertInitialized() {
    if (!this.nodeEngine) throw new Error('Engine state is not initialized')
  }
}

module.exports.Wrapper = Wrapper
