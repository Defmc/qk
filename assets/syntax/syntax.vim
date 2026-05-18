" Vim/Neovim syntax highlighting for the quark programming language
" Language: lang
" Maintainer: generated
" Place this file in ~/.config/nvim/syntax/lang.vim (Neovim)
" or ~/.vim/syntax/lang.vim (Vim)
" Then set filetype with: au BufRead,BufNewFile *.lang set filetype=lang

if exists("b:current_syntax")
  finish
endif

" ─── Keywords ────────────────────────────────────────────────────────────────
syn keyword langKeyword     with match from as this type ref
syn keyword langKeyword     todo

" ─── Namespace-qualified identifiers (e.g. grammar::Expr, lexer::Or) ─────────
" Highlight the namespace prefix separately from the member
syn match   langNamespace   /\<\w\+\ze::/
syn match   langOperator    /::/   contained

" ─── Built-in lang:: grammar/lexer primitives ────────────────────────────────
syn keyword langPrimitive   contained
    \ Expr Atom Product
    \ Or Literal

" Qualified form: grammar::Expr, lexer::Or, etc.
syn match   langQualPrimitive /\<\(grammar\|lexer\)::\(Expr\|Atom\|Product\|Or\|Literal\|dump\|const\)\>/
    \ contains=langNamespace

" ─── Type variables (single lowercase greek or latin letter) ─────────────────
" Greek letters used as type vars: α β γ etc.
syn match   langTypeVar     /\<[α-ωΑ-Ω]\+\>/
" Single-char latin type vars when used after a definition head
syn match   langTypeVar     /\<[a-z]\ze\s*[=:()/\\|]\>/

" ─── Operators ────────────────────────────────────────────────────────────────
syn match   langOperator    /=>/       " lambda / case arrow
syn match   langOperator    /⊕/        " type-level sum constructor
syn match   langOperator    /⊗/        " product type constructor
syn match   langOperator    /∈/        " type-level trait constriant
syn match   langOperator    /λ/        " lambda function
syn match   langOperator    /|/        " match arm separator
syn match   langOperator    /\./       " method call dot
syn match   langOperator    /=/        " definition
syn match   langOperator    /:/        " type annotation
syn match   langOperator    /*/        " deferences

" ─── Strings / Literal token types ───────────────────────────────────────────
syn region  langString      start=/"/ end=/"/ oneline

" ─── Comments (C-style, adjust if your language differs) ─────────────────────
syn region  langBlockComment start=/#/ end=/#\|$/

" ─── Identifiers ─────────────────────────────────────────────────────────────
" Type/constructor names start with uppercase
syn match   langType        /\<[A-Z][A-Za-z0-9_]*\>/
" Term-level identifiers start with lowercase
syn match   langIdent       /\<[a-z_][A-Za-z0-9_]*\>/

syn keyword langBuiltin    const

" ─── Highlight links ─────────────────────────────────────────────────────────
hi def link langKeyword         Keyword
hi def link langBuiltin         Special
hi def link langNamespace       PreProc
hi def link langQualPrimitive   Type
hi def link langType            Type
hi def link langTypeVar         Identifier
hi def link langIdent           Normal
hi def link langOperator        Operator
hi def link langString          String
hi def link langComment         Comment
hi def link langBlockComment    Comment

let b:current_syntax = "qk"
