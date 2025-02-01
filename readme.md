# Linquebot RS

琳酱新版本，由<ruby>锈<rt>Rust</rt></ruby>强力驱动！

WIP

## Install Dependencies

- waife module: [graphviz](https://graphviz.org/) and noto fonts

```bash
sudo apt install graphviz -y
sudo apt install -y --force-yes --no-install-recommends fonts-noto fonts-noto-cjk fonts-noto-cjk-extra fonts-noto-color-emoji ttf-ancient-fonts
```

## Run

```powershell
$env:TELOXIDE_TOKEN="1234567890:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" && cargo run
```

```bash
TELOXIDE_TOKEN="1234567890:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" cargo run
```

## License

列出琳酱使用的第三方开源组件的许可证：

`src/assets/idiom.json`: modified from https://github.com/crazywhalecc/idiom-database/blob/master/data/idiom.json

```
MIT License

Copyright (c) 2021 Whale

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
