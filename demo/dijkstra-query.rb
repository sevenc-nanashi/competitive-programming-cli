# Directed edges with nonnegative weights; queries ask for distance from vertex 1.
n, m, q = gets.split.map(&:to_i)
graph = Array.new(n) { [] }
m.times do
  from, to, cost = gets.split.map(&:to_i)
  graph[from - 1] << [to - 1, cost]
end

distance = Array.new(n, Float::INFINITY)
distance[0] = 0
remaining = (0...n).to_a
# ponytail: O(n²) selection for tiny demo graphs; use a heap for large inputs.
until remaining.empty?
  vertex = remaining.min_by { |i| distance[i] }
  remaining.delete(vertex)
  graph[vertex].each do |to, cost|
    distance[to] = [distance[to], distance[vertex] + cost].min
  end
end

q.times do
  answer = distance[gets.to_i - 1]
  puts answer.infinite? ? 0 : answer # Bug: unreachable vertices should print -1.
end
