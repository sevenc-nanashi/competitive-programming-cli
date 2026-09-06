# frozen_string_literal: true

require "fileutils"
require "tmpdir"

Dir.chdir(File.expand_path("..", __dir__))
system(
  "cargo",
  "build",
  "--locked",
  "--features",
  "mock",
  "--target-dir",
  "target",
  exception: true
)

Dir.mktmpdir("cpg-demo-") do |directory|
  config = File.join(directory, "config")
  cookies = File.join(directory, "cookies")
  workspace = File.join(directory, "workspace")
  bin = File.join(directory, "bin")
  FileUtils.mkdir_p(
    [File.join(config, "problem_template"), cookies, workspace, bin]
  )
  FileUtils.cp("target/debug/cpg", File.join(bin, "cpg"))
  mock = File.join(directory, "mock_service")
  FileUtils.mkdir_p(mock)
  %w[service.toml cookies.txt problems contests].each do |name|
    FileUtils.cp_r(File.join("mock_service", name), mock)
  end
  FileUtils.cp(File.join(mock, "cookies.txt"), File.join(cookies, "mock.txt"))
  File.write(File.join(config, "config.toml"), <<~TOML)
    root = '#{workspace}'
    [language.ruby]
    extensions = ["rb"]
    run = "ruby {input}"
    [language.ruby.submit]
    mock = "ruby"
  TOML
  File.write(File.join(config, "problem_template", "solution.rb"), "puts 0\n")

  system(
    {
      "PATH" => "#{bin}:#{ENV.fetch("PATH")}",
      "CPG_CONFIG_HOME" => config,
      "CPG_COOKIES_HOME" => cookies,
      "CARGO_MANIFEST_DIR" => directory,
      "CPG_DEMO_WORKSPACE" => workspace,
      "CPG_DEMO_CAST" => File.join(directory, "demo.cast"),
      "XDG_CONFIG_HOME" => File.join(directory, "xdg-config"),
      "BASH_ENV" => nil,
      "PROMPT_COMMAND" => nil,
      "NO_COLOR" => nil,
      "FORCE_COLOR" => nil
    },
    "vhs",
    "demo/demo.tape",
    exception: true
  )
  system(
    "agg",
    File.join(directory, "demo.cast"),
    File.join(directory, "demo.gif"),
    exception: true
  )
  FileUtils.cp(
    %w[demo.gif demo.cast].map { |name| File.join(directory, name) },
    "docs/public"
  )
end
