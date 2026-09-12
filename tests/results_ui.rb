# frozen_string_literal: true
# Invoked by the Rust mock workflow with its isolated workspace and cookies.
require 'pty'
require 'io/console'
require 'timeout'
require 'tempfile'
require 'tmpdir'
require 'rbconfig'

# Detach from the invoking terminal so crossterm reads the test PTY's size.
Process.setsid

def check(condition, message)
  raise message unless condition
end

class Terminal
  attr_reader :output

  def initialize(*args, env: {}, columns: 180)
    @master, @slave = PTY.open
    @master.winsize = [24, columns]
    @mode = mode
    @errors = Tempfile.new('cpg-results-errors')
    @output = +''
    @pid = Process.spawn(
      { 'TERM' => 'xterm-256color', 'NO_COLOR' => nil, 'FORCE_COLOR' => nil }.merge(env),
      *args, in: @slave, out: @slave, err: @errors
    )
    yield self
  ensure
    if @pid
      Process.kill('KILL', @pid)
      Process.wait(@pid)
    end
    @master&.close
    @slave&.close
    @errors&.close!
  end

  def mode
    IO.popen(['stty', '-g'], in: @slave, &:read).strip
  end

  def read_for(seconds)
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + seconds
    loop do
      remaining = deadline - Process.clock_gettime(Process::CLOCK_MONOTONIC)
      break if remaining <= 0 || !IO.select([@master], nil, nil, remaining)
      @output << @master.read_nonblock(65_536)
      break if block_given? && yield
    end
  rescue IO::WaitReadable
    retry
  end

  def expect(text)
    read_for(8) { @output.include?(text) } unless @output.include?(text)
    check(@output.include?(text), "Missing #{text.inspect}: #{@output.inspect}")
  end

  def send(text)
    @output.clear
    @master.write(text)
  end

  def resize(rows, columns)
    @output.clear
    @master.winsize = [rows, columns]
    Process.kill('WINCH', @pid)
  end

  def interrupt
    Process.kill('INT', @pid)
  end

  def finish(code, ui: true)
    _, status = Timeout.timeout(8) { Process.wait2(@pid) }
    @pid = nil
    read_for(0.1)
    check(status.exitstatus == code, "Exit #{status}: #{File.read(@errors.path)}")
    check(mode == @mode, 'Terminal mode was not restored')
    if ui
      check(@output.include?("\e[?1049l") && @output.include?("\e[?25h"),
            'Alternate screen or cursor was not restored')
    end
  end
end

binary, stored = ARGV
original = File.read(stored)
expected_url = original.match(/^url = "([^"]+)"$/)[1]
set_status = lambda do |status|
  File.write(stored, original.sub(/^status = "[^"]+"$/, "status = \"#{status}\""))
end

begin
  [
    ['AC', 32, ['cat'], 0],
    ['WA', 33, ['true'], 1],
    ['RE', 35, ['false'], 1]
  ].each do |verdict, color, command, code|
    Terminal.new(binary, 'test', '--', *command) do |term|
      term.finish(code, ui: false)
      check(term.output.include?("\e[#{color}m#{verdict}\e[0m"), "Missing #{verdict} color")
    end
  end

  interactive_args = [
    'test', '--interactive', '--show-io', 'always', '--time-limit', '2000', '--judge',
    %q{sh -c 'printf "question\n"; IFS= read -r answer; test "$answer" = answer'},
    '--', 'sh', '-c',
    %q{IFS= read -r question; test "$question" = question || exit 1; printf "answer\n"}
  ]
  Terminal.new(binary, *interactive_args) do |term|
    term.finish(0, ui: false)
    check(term.output.include?("\e[32m< \e[0mquestion"), 'Judge color must stop before output')
    check(term.output.include?("\e[33m> \e[0manswer"), 'Solution color must stop before output')
    plain = term.output.gsub(/\e\[[\d;]*m/, '')
    check(plain.include?("< question\r\n> answer\r\n"), 'Extra interaction newlines')
    check(!plain.include?('(no eol)'), 'Colored interaction lost its final newline')
  end

  Terminal.new(binary, 'results') do |term|
    term.finish(0, ui: false)
    check(term.output.include?("\e[32m"), 'Missing AC color with redirected stderr')
    check(!term.output.include?("\t"), 'Terminal listing still uses tabs')
    check(term.output.include?("    #{expected_url}"), 'Missing indented URL')
  end

  Terminal.new(binary, 'test', '-n', *interactive_args.drop(1)) do |term|
    term.finish(0, ui: false)
    plain = term.output.gsub(/\e\[[\d;]*m/, '')
    check(plain.include?("0 < question\r\n1 > answer\r\n"), 'Wrong exchange numbers')
    check(term.output.include?("0 \e[32m<\e[0m question"), 'Wrong numbered judge color boundary')
    check(term.output.include?("1 \e[33m>\e[0m answer"), 'Wrong numbered solution color boundary')
  end

  [[], ['-n']].each do |number_flags|
    Terminal.new(binary, 'test', '-p', 'outputs', *number_flags, *interactive_args.drop(1)) do |term|
      term.finish(0, ui: false)
      check(term.output.match?(/\e\[32m(?:\e\[1m)?Judge:/), 'Missing judge header color')
      check(term.output.match?(/\e\[33m(?:\e\[1m)?Solution:/), 'Missing solution header color')
      judge_separator, solution_separator = if number_flags.empty?
        ["\e[32m>\e[0m", "\e[33m<\e[0m"]
      else
        ["\e[32m| 0\e[0m :", ": \e[33m1 |\e[0m"]
      end
      check(term.output.match?(/(?:\A|\n)question +#{Regexp.escape(judge_separator)}/),
            'Judge pane output and colon must be uncolored')
      check(term.output.include?("#{solution_separator} answer\r\n"),
            'Solution pane output and colon must be uncolored')
    end
  end

  Dir.mktmpdir('cpg-panes') do |directory|
    File.write(File.join(directory, 'case.in'), "abcdefghijklmnopqrstuv\n")
    File.write(File.join(directory, 'case.out'), "ok\n")
    args = ['test', '-d', directory, '-p', 'all', '-n', '--', 'printf', "abcdefghijklmnopqrstuvwxy\n"]
    [40, 15].each do |columns|
      Terminal.new(binary, *args, columns: columns) do |term|
        term.finish(columns == 40 ? 1 : 2, ui: false)
        next if columns == 15
        plain = term.output.gsub(/\e\[[\d;]*m/, '')
        rows = plain.lines.map(&:chomp).select { |line| line.include?(' | ') || line.include?(' : ') }
        check(rows.all? { |line| line.length == 40 }, 'Panes exceeded terminal width')
        check(rows.any? { |line| line.start_with?('1 | uv') && line.include?('ok') && line.end_with?('abcdefghij') },
              'Wrapped input and outputs are misaligned')
      end
    end
    File.write(File.join(directory, 'case.out'), "same good end\n")
    %w[none outputs all].each do |panes|
      %w[line word].each do |highlight|
        args = ['test', '-d', directory, '-p', panes, '-H', highlight, '--', 'printf', "same bad end\n"]
        Terminal.new(binary, *args, columns: 38) do |term|
          term.finish(1, ui: false)
          color = nil
          bold = false
          highlighted = { 31 => +'', 32 => +'' }
          term.output.scan(/\e\[([\d;]*)m|([^\e]+)/) do |codes, text|
            if codes
              codes.split(';').map(&:to_i).each do |code|
                color, bold = nil, false if code.zero?
                color = code if [31, 32].include?(code)
                bold = true if code == 1
                check(code != 7 && !(40..49).cover?(code) && !(100..107).cover?(code),
                      'Highlight used reverse video or a background color')
              end
            elsif !bold && highlighted.key?(color)
              highlighted[color] << text
            end
          end
          check(highlighted[32] == (highlight == 'line' ? 'same good end' : 'good'), 'Wrong expected highlight')
          check(highlighted[31] == (highlight == 'line' ? 'same bad end' : 'bad'), 'Wrong actual highlight')
        end
        [ [['--no-color'], {}], [[], { 'NO_COLOR' => '1' }] ].each do |flags, env|
          Terminal.new(binary, *flags, *args, env: env) do |term|
            term.finish(1, ui: false)
            check(!term.output.include?("\e"), 'Highlight colors were not disabled')
          end
        end
      end
    end
  end

  [ [['--no-color'], {}], [[], { 'NO_COLOR' => '1' }] ].each do |flags, env|
    Terminal.new(binary, *flags, 'test', '-n', *interactive_args.drop(1), env: env) do |term|
      term.finish(0, ui: false)
      check(term.output.include?('< question') && term.output.include?('> answer') &&
            !term.output.include?("\e"), 'Interaction colors were not disabled')
    end
    Terminal.new(binary, 'test', *flags, '-p', 'all', '-n', '-v', 'always', '--', 'cat', env: env) do |term|
      term.finish(0, ui: false)
      check(term.output.include?('sample-1: AC (') && !term.output.include?("\e"),
            'Test verdict colors were not disabled')
    end
    Terminal.new(binary, 'results', *flags, env: env) do |term|
      term.finish(0, ui: false)
      check(!term.output.include?("\e"), 'Colors were not disabled')
    end
    Terminal.new(binary, 'results', '--ui', *flags, env: env) do |term|
      term.expect('Running')
      check(!term.output.match?(/\e\[[\d;]*m/), 'TUI colors were not disabled')
      term.send('q')
      term.finish(0)
    end
  end

  Dir.mktmpdir('cpg-open') do |directory|
    opened = File.join(directory, 'url')
    opener = File.join(directory, 'xdg-open')
    File.write(opener, "#!#{RbConfig.ruby}\nFile.write(ENV.fetch('CPG_OPENED_URL'), ARGV.fetch(0))\n")
    File.chmod(0o755, opener)
    env = { 'PATH' => directory, 'CPG_OPENED_URL' => opened }
    Terminal.new(binary, 'results', '--ui', '--limit', '2', env: env) do |term|
      term.expect('! Refreshing |')
      term.expect('Running')
      term.read_for(0.5)
      spinner_frames = term.output.scan(/([|\/\\-]) Running    \|/).flatten.uniq
      check(spinner_frames.length > 1, 'Running spinner did not animate')
      check(term.output.include?("\e[?1049h"), 'No alternate screen')
      check(term.output.include?("\e[32m"), 'No TUI AC color')
      set_status.call('WA')
      term.send('')
      term.expect("\e[33m")
      term.send('p')
      term.expect('* Paused     |')
      set_status.call('TLE')
      term.send('')
      term.read_for(3.4)
      check(!term.output.include?('TLE'), 'Pause did not stop polling')
      term.send('r')
      term.expect("\e[95m")
      check(term.output.include?('Paused'), 'Manual refresh should remain paused')
      term.resize(1, 1)
      term.read_for(0.2)
      term.resize(4, 100)
      term.expect('Paused')
      term.send("\e[A")
      term.expect("\e[32m")
      term.send("\e[B")
      term.expect("\e[95m")
      term.resize(24, 180)
      term.expect('Paused')
      term.send('1')
      term.expect('Opened submission in browser')
      check(File.read(opened) == expected_url, 'Opened the wrong submission')
      term.send('p')
      term.expect('Running')
      term.send('q')
      term.finish(0)
    end
  end

  [:keyboard, :signal].each do |interrupt|
    Terminal.new(binary, 'results', '--ui') do |term|
      term.expect('Running')
      interrupt == :keyboard ? term.send("\x03") : term.interrupt
      term.finish(130)
    end
  end

  Terminal.new(binary, 'results', '--ui') do |term|
    term.expect('Running')
    File.write(stored, 'invalid = [')
    term.send('r')
    term.finish(2)
  end
ensure
  File.write(stored, original)
end
