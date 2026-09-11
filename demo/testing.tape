# Run from the repository root with `mise run demo demo/testing.tape`.
Require cpg
Require asciinema
Require ruby
Require sed

Set Shell bash
Set FontFamily "DejaVu Sans Mono"
Set FontSize 20
Set Width 1100
Set Height 500
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type@1ms `export PS1='$ '; cd "$CPG_DEMO_WORKSPACE";`
Enter
Wait+Line /\$$/
Type@1ms `cpg download https://mock.local/problems/dijkstra-query`
Enter
Wait+Line /\$$/
Type@1ms `cp "$CPG_DEMO_WORKSPACE/dijkstra-query.rb" "$CPG_DEMO_WORKSPACE/mock/problems/dijkstra-query/solution.rb"; cd "$CPG_DEMO_WORKSPACE/mock/problems/dijkstra-query"; clear`
Enter
Type@1ms `asciinema rec --quiet --overwrite --return --output-format asciicast-v3 --title 'cpg test: Dijkstra Query' --command 'bash --noprofile --norc' "$CPG_DEMO_DESTINATION"`
Enter
Wait+Line /\$$/
Show

Type "# Dijkstra Query: shortest distances from vertex 1; unreachable = -1"
Enter

Wait+Line /\$$/
Type "bat ./test/sample-1.in"
Enter

Wait+Line /\$$/
Sleep 1s
Type "bat ./test/sample-1.out"
Enter

Wait+Line /\$$/
Sleep 1s
Type "cpg test solution.rb --fast-fail"
Enter
Wait+Line /\$$/
Sleep 3s

Ctrl+L
Type "# Compare outputs with query numbers"
Enter
Type "cpg test solution.rb --panes outputs --line-numbers --highlight word"
Enter
Wait+Line /\$$/
Sleep 5s

Ctrl+L
Type "# You can also show the input next to the output"
Enter
Type "cpg test solution.rb -p 3 -n -H word"
Enter
Wait+Line /\$$/
Sleep 5s

Ctrl+L
Type "# Fix the unreachable distance, then run both cases in parallel"
Enter
Type "sed -i 's/? 0 :/? -1 :/' solution.rb"
Enter
Wait+Line /\$$/
Type "cpg test solution.rb --show-io never --jobs 2"
Enter
Wait+Line /\$$/
Sleep 3s

Ctrl+L
Type "# You can also show the input and output for passing test cases"
Enter
Type "cpg test solution.rb -p all -n --show-io always"
Enter
Wait+Line /\$$/
Sleep 5s

Ctrl+D
