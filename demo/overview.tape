# Run from the repository root with `mise run demo`.
# Wait for prompts: VHS 0.10.0's Wait+Screen reads old lines after scrolling.
Require cpg
Require asciinema
Require ruby

Set Shell bash
Set FontFamily "DejaVu Sans Mono"
Set FontSize 20
Set Width 1100
Set Height 500
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type@1ms `export PS1='$ '; cd "$CPG_DEMO_WORKSPACE"; asciinema rec --quiet --overwrite --return --output-format asciicast-v3 --title 'cpg demo' --command 'bash --noprofile --norc' "$CPG_DEMO_DESTINATION"`
Enter
Wait+Line /\$$/
Show

Type "eza -T $CPG_CONFIG_HOME"
Enter
Wait+Line /\$$/
Sleep 1s

Type "bat $CPG_CONFIG_HOME/config.toml"
Enter
Wait+Line /\$$/
Sleep 1s

Type "bat $CPG_CONFIG_HOME/problem_template/solution.rb"
Enter
Wait+Line /\$$/
Sleep 1s

Ctrl+L

Type "cpg prepare https://mock.local/contests/practice"
Enter
Wait+Line /\$$/
Sleep 2s

Type 'cd "$(cpg config --root)/mock/contests/practice/1_sum"'
Enter
Wait+Line /\$$/
Sleep 1s

Type "eza -T ."
Enter
Wait+Line /\$$/
Sleep 1s

Ctrl+L
Type "bat solution.rb"
Enter
Wait+Line /\$$/
Sleep 1s
Type "cpg test solution.rb --fast-fail"
Enter
Wait+Line /\$$/
Sleep 3s

Type `vim solution.rb`
Enter
Sleep 1s
Type@100ms `ggVGc`
Type 'puts gets.chomp.split.map(&:to_i).sum'
Sleep 500ms
Escape
Type@100ms `:wq`
Enter
Wait+Line /\$$/

Sleep 1s
Type "cpg test solution.rb"
Enter
Wait+Line /\$$/
Sleep 3s

Type "cpg submit solution.rb"
Enter
Wait+Line /\$$/
Sleep 2s
Type "cpg results --ui"
Sleep 1s
Enter
Wait+Screen /Running/
Sleep 10s
Type "q"
Wait+Line /\$$/
Sleep 1s
Ctrl+D

Hide
Type "exit"
Enter
