# Guess the secret number in 1..10.
$stdout.sync = true
secret = rand(1..10)
puts "1 10"
while (line = $stdin.gets)
  match = /\A([?!]) (\d+)\n\z/.match(line)
  abort "Expected ? number or ! number" unless match
  number = Integer(match[2])
  abort "Number outside 1..10" unless (1..10).cover?(number)
  if match[1] == "!"
    exit(number == secret ? 0 : 1)
  end
  puts number < secret ? "HIGHER" : number > secret ? "LOWER" : "EQUAL"
end
abort "Missing final answer"
