Gem::Specification.new do |s|

  target_platform = -> { ENV['YGG_BUILD_PLATFORM'] || Gem::Platform::CURRENT }

  s.name = 'yggdrasil-engine'
  s.version = '1.1.1'
  s.summary = 'Unleash engine for evaluating feature toggles'
  s.description = '...'
  s.authors = ['Unleash']
  s.email = 'liquidwicked64@gmail.com'
  s.files = Dir.glob("{lib,spec}/**/*") + ["README.md"] + Dir["lib/**/*"]
  s.homepage = 'http://github.com/username/my_gem'
  s.license = 'MIT'
  s.add_dependency "ffi", "~> 1.17.2"
  s.platform = target_platform.call
  s.metadata["yggdrasil_core_version"] = '0.18.2'
end
