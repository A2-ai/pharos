// Minimal stub so R's build system produces a shared library.
// All entry points (R_init_*) are defined in Rust via miniextendr_init!().
//
// The extern reference below forces the linker to pull in the archive member
// containing R_init_<pkg> from the Rust staticlib. With codegen-units = 1,
// that single member contains the entire user crate — including all linkme
// distributed_slice entries. Without this reference, the linker would extract
// nothing from the archive (member extraction is driven by undefined symbols).
//
// miniextendr_force_link is a package-independent symbol emitted by
// miniextendr_init!(), so this file works for any package without substitution.
extern const char miniextendr_force_link;
const void *miniextendr_anchor = &miniextendr_force_link;
