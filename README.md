[![Build Status](https://travis-ci.org/dan-t/clang-typecheck.svg?branch=master)](https://travis-ci.org/dan-t/clang-typecheck)
[![](http://meritbadge.herokuapp.com/clang-typecheck)](https://crates.io/crates/clang-typecheck)

clang-typecheck
===============

A command line tool to type check a C++ source file with a [clang compilation database](http://clang.llvm.org/docs/JSONCompilationDatabase.html). 

The database contains the compiler commands - with all flags, defines and
includes - of all source files of the project. The easiest way to get
a database is for a [cmake](https://cmake.org/) build project by calling
`cmake` with the option `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`. After the
complete rebuild of the project the root of the build directory will
contain a database named `compile_commands.json`.

`clang-typecheck` extracts the compiler command for the given source file
from the database, executes it and outputs the output of the compiler.

The design of `clang-typecheck` was to get the most minimal program,
that doesn't need any configuration and should just work.

There're several programs operating with a database and also doing type
checking, like [rtags](https://github.com/Andersbakken/rtags) or
[YouComplete](https://github.com/Valloric/YouCompleteMe), but either - in the
case of rtags - they do the type checking asynchronously, which makes it harder
to integrate into several editors or - in the case of YouCompleteMe - they feel
quite a bit heavyweight, are harder to configure and slow done [vim](http://www.vim.org/) quite a bit.

Another issue is, that these programs sometimes use [clang](http://clang.llvm.org/)
for the type checking and not the compiler used in the database, which might
give different warnings for the type checking and the building.

`clang-typecheck` isn't the best fit for on the fly type checking -
here the asynchronously solutions are more appropriate - it is meant for
synchronous on demand type checking - by pressing some editor shortcut -
with minimal hassle to configure.

Installation
------------

`clang-typecheck` is build with [Rust](https://www.rust-lang.org/en-US/) so at least
`rustc` and `cargo` are needed to build it.

The easiest way to get both is by using `rustup`:

    $> curl https://sh.rustup.rs -sSf | sh

After this call you should have a `rustc` and `cargo` binary available at
`~/.cargo/bin/`, so adding this path to the `PATH` enviroment variable is
recommendable.

For non unix like platforms take a look at [here](https://github.com/rust-lang-nursery/rustup.rs/#other-installation-methods).

And now building and installing `clang-typecheck`:

    $> cargo install clang-typecheck

The build binary will be located at `~/.cargo/bin/clang-typecheck`.

Usage
-----

Type checking a source file with a database:

    $> clang-typecheck  /absolute_path_to/SomeSource.cpp  path_to/compile_commands.json

This will look up `SomeSource.cpp` in `compile_commands.json`, executes the
compiler command and outputs the output of the compiler. 

This makes it possible to use `clang-typecheck` as a compiler replacement in editors
that parse the output of the compiler and display the errors.

Text Editor Integration
-----------------------

[vim-clang-typecheck](<https://github.com/dan-t/vim-clang-typecheck>)

Possible Issues
---------------

The compiler commands for source files are cached at `~/.clang_typecheck/cache/cmds`, so
that multiple type checks of the same source don't need to look up the command again
in the database. Normally this shouldn't be an issue, because the commands in the database
very rarely change in a way that affects type checking, but if there're problems, then
the cache at `~/.clang_typecheck/cache/cmds` can be just cleared.

Currently only the [gcc](https://gcc.gnu.org/) and [clang](http://clang.llvm.org/) compilers
are supported, because to prevent the object file creation and only apply the type checking
the flag `-fsyntax-only` is appended to the compiler command, which is known by both.

