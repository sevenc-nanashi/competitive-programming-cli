# Competitive Programming CLI (cpcli)

> [!WARNING]
> This project is in early development.
>
> - Configuration and metadata formats may change.
> - Only Linux is supported for now.

cpcli is a command-line interface tool for competitive programming.

This tool can:

- Download a problem from various online judges.
- Download multiple problems from a contest.
- Compile and test solutions, including custom and interactive judges.
- Generate test cases and reference answers.
- Submit solutions to judges.
- Watch submission results.
- List problems and contests you've downloaded.

Currently supported online judges:

- AtCoder
- AtCoder Problems (Virtual Contests)
- Yukicoder

cpcli currently supports Linux. Building requires Rust 1.91 or newer.

## Documentation

See the [documentation site](https://sevenc-nanashi.github.io/competitive-programming-cli/)
for installation, configuration, and command usage. The Markdown sources are
available in [docs/content](docs/content/index.md).

## Acknowledgements

This tools is heavily inspired by following tools:

- [online-judge-tools/oj](https://github.com/online-judge-tools/oj)
- [online-judge-tools/template-generator](https://github.com/online-judge-tools/template-generator)

```
MIT License

Copyright (c) 2020 Kimiyuki Onaka

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
