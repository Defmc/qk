#!/bin/sh

echo "WARN: run this script while in the project root. Like ~/qk. NOT ON THE {assets/sh,sh} FOLDER."

local_syn_vim_dir="$HOME/.config/nvim/syntax"
local_ftd_vim_dir="$HOME/.config/nvim/ftdetect"

mkdir -p $local_syn_vim_dir
mkdir -p $local_ftd_vim_dir

syntax_dir="$(pwd)/assets/syntax"
ln -sf "$syntax_dir/syntax.vim" "$local_syn_vim_dir/qk.vim"
ln -sf "$syntax_dir/ftdetect.vim" "$local_ftd_vim_dir/qk.vim"

echo "OKAY: done/updated"
