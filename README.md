# C to C3 transpiler

This transpiler intends to do a best-effort conversion from C to C3. It's not perfect, you *will* most likely have to edit the final output.
It's still highly work-in-progress and incomplete. The code could be much better, as right now it's a giant 1000+ line mess. It's a start though.

This transpiler uses tree-sitter for parsing C code. Written in Rust as that's a language I'm decently comfortable with, and it has tree-sitter.