#!/bin/env fish
set -l distro (lsb-release -is | string lower | string trim)
if [ $distro = fedora ]
    set -gx BINDGEN_EXTRA_CLANG_ARGS "-I /usr/lib/clang/20/include"
end
cargo run
