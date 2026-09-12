$stdout.sync = true
low, high = gets.split.map(&:to_i)
loop do
  middle = (low + high) / 2
  puts "? #{middle}"
  case gets&.chomp
  when "HIGHER" then low = middle + 1
  when "LOWER" then high = middle - 1
  when "EQUAL"
    puts "! #{middle}"
    break
  else abort "Unexpected judge response"
  end
end
