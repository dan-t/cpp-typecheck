[![Build Status](https://travis-ci.org/dan-t/cpp-typecheck.svg?branch=master)](https://travis-ci.org/dan-t/cpp-typecheck)
[![](http://meritbadge.herokuapp.com/cpp-typecheck)](https://crates.io/crates/cpp-typecheck)

cpp-typecheck
=============

A command line tool to type check a C++ source file with a [clang compilation database](http://clang.llvm.org/docs/JSONCompilationDatabase.html). 

`cpp-typecheck` extracts the compiler command for the given source file
from the database, executes it and outputs the output of the compiler.

The design of `cpp-typecheck` was to get the most minimal program,
that doesn't need any configuration, should just work and is easy to
integrate into editors.

The database contains the compiler commands - with all flags, defines and
includes - of all source files of the project. The easiest way to get
a database is for a [cmake](https://cmake.org/) build project by calling
`cmake` with the option `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`. After the
complete rebuild of the project the root of the build directory will
contain a database named `compile_commands.json`.

There're several programs operating with a database and also doing type
checking, like [rtags](https://github.com/Andersbakken/rtags) or
[YouCompleteMe](https://github.com/Valloric/YouCompleteMe), but either - in the
case of rtags - they do the type checking asynchronously, which makes it harder
to integrate into several editors or - in the case of YouCompleteMe - they feel
quite a bit heavyweight, are harder to configure and slow done my prefered editor
[vim](http://www.vim.org/) quite a bit.

Another issue is, that these programs sometimes use [clang](http://clang.llvm.org/)
for the type checking and not the compiler used in the database, which might
give different warnings for the type checking and the building, which sometimes
isn't the desired behaviour.

`cpp-typecheck` isn't the best fit for on the fly type checking -
here the asynchronously solutions are more appropriate - it is meant for
synchronous on demand type checking - by pressing some editor shortcut -
with minimal hassle to configure.

Installation
------------

`cpp-typecheck` is build with [Rust](https://www.rust-lang.org/en-US/) so at least
`rustc` and `cargo` are needed to build it.

The easiest way to get both is by using [rustup](https://www.rustup.rs/):

    $> curl https://sh.rustup.rs -sSf | sh

After this call you should have a `rustc` and `cargo` binary available at
`~/.cargo/bin/`, so adding this path to the `PATH` enviroment variable is
recommendable.

For non unix like platforms take a look at [here](https://github.com/rust-lang-nursery/rustup.rs/#other-installation-methods).

And now building and installing `cpp-typecheck`:

    $> cargo install cpp-typecheck

The build binary will be located at `~/.cargo/bin/cpp-typecheck`.

Usage
-----

Type checking a source file:

    $> cpp-typecheck  /absolute_path_to/SomeSource.cpp

This will search for a database named `compile_commands.json` upwards the directory
tree starting at the directory `/absolute_path_to/`. Then `SomeSource.cpp` is
looked up in the database, the compiler command is executed and the compiler output
is output.

This makes it possible to use `cpp-typecheck` as a compiler replacement in editors
that parse the output of the compiler and display the errors.

If the database isn't reachable through the source file directory then the database
has also to be given:

    $> cpp-typecheck  /absolute_path_to/SomeSource.cpp  path_to/compile_commands.json

It's also possible to use an other compiler for the type checking than the one defined
in the database, as long as the compiler arguments are compatible (which is the case for gcc and clang):

    $> cpp-typecheck  --compiler clang  /absolute_path_to/SomeSource.cpp

Text Editor Integration
-----------------------

[vim-cpp-typecheck](<https://github.com/dan-t/vim-cpp-typecheck>)

Possible Issues
---------------

The compiler commands for source files are cached at `~/.cpp_typecheck/cache/cmds`, so
that multiple type checks of the same source don't need to look up the command again
in the database. Normally this shouldn't be an issue, because the commands in the database
very rarely change in a way that affects type checking, but if there're problems, then
the cache at `~/.cpp_typecheck/cache/cmds` can be just cleared.

There's also the option `--no-cache` to ignore the cache and to always lookup the
compiler command in the database.
