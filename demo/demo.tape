# Run from the repository root with `mise run demo`.
Output docs/public/demo.gif

Require cpg
Require asciinema
Require ruby

Set Shell bash
Set FontFamily "DejaVu Sans Mono"
Set FontSize 20
Set Width 1100
Set Height 700
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type@1ms `export PS1='$ '; cd "$CPG_DEMO_WORKSPACE"; asciinema rec --quiet --overwrite --return --output-format asciicast-v2 --title 'cpg demo' --command 'bash --noprofile --norc' "$CPG_DEMO_CAST"`
Enter
Wait+Line /\$$/
Show

Type "cpg prepare https://mock.local/contests/practice"
Enter
Wait+Screen /Created workspace:/
Wait+Line /\$$/
Sleep 2s

Type 'cd "$(cpg config --root)/mock/contests/practice/1_sum"'
Enter
Wait+Line /\$$/
Sleep 1s

Type "pwd"
Enter
Wait+Line /\$$/
Sleep 1s

Ctrl+L
Type "cat solution.rb"
Enter
Wait+Line /\$$/
Sleep 1s
Type "cpg test solution.rb --fast-fail"
Enter
Wait+Screen /WA/
Wait+Line /\$$/
Sleep 3s

Type `echo 'puts gets.chomp.split.map(&:to_i).sum' > solution.rb`
Enter
Wait+Line /\$$/
Sleep 1s
Ctrl+L

Type "cpg test solution.rb -j 2"
Enter
Wait+Screen /AC/
Wait+Line /\$$/
Sleep 3s

Type "cpg submit solution.rb"
Enter
Wait+Screen /Submitted [0-9]+:/
Wait+Line /\$$/
Sleep 2s
Type "cpg results --ui"
Enter
Wait+Screen /Running/
Sleep 7s
Type "q"
Wait+Line /\$$/
Sleep 1s

Hide
Type "exit"
Enter
Wait+Line /\$$/
Sleep 500ms
