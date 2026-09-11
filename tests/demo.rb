# frozen_string_literal: true

require "open3"

Dir.chdir(File.expand_path("..", __dir__))
source = File.read("demo/dijkstra-query.rb")
fixed = source.sub("? 0 :", "? -1 :")
raise "Demo fix no longer applies" if source == fixed

Dir.glob("mock_service/problems/dijkstra-query/test/*.in").each do |path|
  input = File.read(path)
  expected = File.read(path.sub(/\.in\z/, ".out"))
  actual, error, status = Open3.capture3("ruby", "-e", fixed, stdin_data: input)
  raise "#{path}: #{error}" unless status.success? && actual == expected
end

actual, error, status = Open3.capture3(
  "ruby", "-e", source,
  stdin_data: File.read("mock_service/problems/dijkstra-query/test/01.in")
)
raise "Expected the demo's unreachable-vertex bug: #{error}" unless status.success? && actual == "0\n3\n5\n0\n"
puts "Demo samples and bug fix passed"
