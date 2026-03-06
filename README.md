## Quark, maximizing functional languages through a micro-language
No external runtime dependencies, nothing beyond lambda calculus.

Quark is a programming language that has the minimal necessary to expand it.

To create a DSL, debug a lambda calculus library or use it as full  programming language. You choose.

The development is extremely rudimental. I've been focused on dividing each stage and make it readable before delving into improving it.
Don't use for anything but fun (yet).

Due it's development, I've found several gimmicks to define a correct language, so each stage required a way to debug, which you can enable anytime you want.

Just so you know where this project heads, here's my goals with it:
### Quark's philosophy:
- The language should enable the expansion of any part of itself.
- - It includes the syntax, behavior and performance.
- What is fundamental, should not change.
- - Once I get to a level where I can call it _at least_ useful, I'll ensure support for that version for a lifetime.
- - It doesn't mean I plan on improving older versions, but that I'll make sure you can somehow use legacy libraries into new projects.
- `qk` will only have what is fundamentally necessary to satisfy the requirements above.
- - It doesn't mean there won't be recommended libraries for each use-case. It means that if you don't like something, feel free to do your own implementation (that's the whole goal of `qk`!).
