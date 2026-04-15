/*
 * mx_abi.h - miniextendr Trait ABI C Interface
 *
 * This header defines the stable C ABI for cross-package trait dispatch.
 * Include this header in packages that need to interact with miniextendr's
 * trait system from C code.
 *
 * STABILITY GUARANTEE:
 * - All types are append-only (fields never removed or reordered)
 * - Function signatures are stable
 * - ABI version is tracked for compatibility checking
 *
 * USAGE FOR CONSUMER PACKAGES:
 *
 * 1. Add to DESCRIPTION file:
 *      LinkingTo: hyperion
 *      Imports: hyperion
 *
 *    LinkingTo makes this header available via:
 *      #include <mx_abi.h>
 *
 *    Imports ensures the package is loaded first (so C-callables are registered).
 *
 * 2. In R_init_<yourpkg>(), load C-callables:
 *
 *      typedef SEXP (*mx_wrap_fn)(mx_erased*);
 *      static mx_wrap_fn p_mx_wrap = NULL;
 *
 *      void R_init_yourpkg(DllInfo *dll) {
 *          p_mx_wrap = (mx_wrap_fn) R_GetCCallable("hyperion", "mx_wrap");
 *          // ... similarly for mx_get, mx_query
 *      }
 *
 * 3. Use via function pointers:
 *      SEXP result = p_mx_wrap(my_erased_ptr);
 *
 * See R-exts section 5.4.3 "Linking to native routines in other packages" for details.
 *
 * THREAD SAFETY:
 * All functions must be called from R's main thread only.
 */

#ifndef MINIEXTENDR_MX_ABI_H
#define MINIEXTENDR_MX_ABI_H

#include <R.h>
#include <Rinternals.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * ABI Version
 * ============================================================================ */

/**
 * Current ABI version number.
 *
 * Increment when making breaking changes:
 * - Major: Breaking layout changes
 * - Minor: New fields/functions (backwards compatible)
 */
#define MX_ABI_VERSION_MAJOR 0
#define MX_ABI_VERSION_MINOR 1

/* ============================================================================
 * Type Tags
 * ============================================================================ */

/**
 * Type tag for runtime type identification.
 *
 * A 128-bit identifier used to identify concrete types and trait interfaces
 * at runtime. Generated from type/trait paths via hashing.
 *
 * LAYOUT: This struct is frozen. Fields will never be reordered.
 */
typedef struct mx_tag {
    uint64_t lo;  /* Lower 64 bits */
    uint64_t hi;  /* Upper 64 bits */
} mx_tag;

/**
 * Create a type tag from two 64-bit values.
 */
static inline mx_tag mx_tag_new(uint64_t lo, uint64_t hi) {
    mx_tag tag;
    tag.lo = lo;
    tag.hi = hi;
    return tag;
}

/**
 * Compare two type tags for equality.
 */
static inline int mx_tag_eq(mx_tag a, mx_tag b) {
    return a.lo == b.lo && a.hi == b.hi;
}

/* ============================================================================
 * Method Signature
 * ============================================================================ */

/**
 * Method signature for trait vtable entries.
 *
 * All trait methods are erased to this uniform signature:
 * - data: Pointer to the concrete object data
 * - argc: Number of arguments in argv
 * - argv: Array of SEXP arguments from R
 * - Returns: SEXP result to R
 *
 * The method shim is responsible for:
 * 1. Checking argc matches expected arity
 * 2. Converting arguments from SEXP
 * 3. Calling the actual method
 * 4. Converting the result to SEXP
 */
typedef SEXP (*mx_meth)(void *data, int argc, const SEXP *argv);

/* ============================================================================
 * Vtable and Erased Object Types
 * ============================================================================ */

/* Forward declaration for base vtable */
typedef struct mx_erased mx_erased;

/**
 * Base vtable present in all erased objects.
 *
 * Provides minimal operations for any erased object:
 * - drop: Destructor for cleanup
 * - concrete_tag: Type tag for downcasts
 * - query: Interface lookup by tag
 *
 * LAYOUT: This struct is frozen. New fields will only be appended.
 */
typedef struct mx_base_vtable {
    /**
     * Destructor called when R's external pointer is garbage collected.
     *
     * @param ptr Pointer to the erased object (not the data)
     */
    void (*drop)(mx_erased *ptr);

    /**
     * Tag identifying the concrete type.
     */
    mx_tag concrete_tag;

    /**
     * Query for interface vtable by tag.
     *
     * @param ptr Pointer to the erased object
     * @param trait_tag Tag identifying the requested trait
     * @return Pointer to vtable if implemented, NULL otherwise
     */
    const void *(*query)(mx_erased *ptr, mx_tag trait_tag);

    /**
     * Byte offset from wrapper struct start to the data field.
     *
     * The wrapper is laid out as: { mx_erased erased; T data; }.
     * When T has stricter alignment than mx_erased, padding exists
     * between erased and data. This field stores the correct offset.
     */
    size_t data_offset;
} mx_base_vtable;

/**
 * Type-erased object header.
 *
 * This is the common prefix of all erased objects. The actual data
 * follows this header in memory.
 *
 * Memory layout:
 * +------------------------+
 * | mx_erased              |
 * |   base ----------------+---> static vtable
 * +------------------------+
 * | (type-specific data)   |
 * +------------------------+
 *
 * LAYOUT: This struct is frozen. New fields will only be appended.
 */
struct mx_erased {
    /**
     * Pointer to the base vtable.
     *
     * Must point to a valid, static vtable for the object's lifetime.
     */
    const mx_base_vtable *base;
};

/* ============================================================================
 * C-Callable Functions
 * ============================================================================
 *
 * These functions are registered with R_RegisterCCallable and can be
 * obtained via R_GetCCallable("hyperion", "mx_*").
 *
 * NOTE: Function bodies are defined in mx_abi.rs (miniextendr-api).
 */

/**
 * Wrap an erased object pointer in an R external pointer.
 *
 * Creates an EXTPTRSXP that wraps the erased object. The finalizer
 * will call the object's drop function when garbage collected.
 *
 * @param ptr Heap-allocated pointer to erased object
 * @return R external pointer (EXTPTRSXP)
 *
 * SAFETY:
 * - ptr must be heap-allocated (will be freed by finalizer)
 * - Must be called on R's main thread
 */
SEXP mx_wrap(mx_erased *ptr);

/**
 * Extract an erased object pointer from an R external pointer.
 *
 * @param sexp R external pointer created by mx_wrap
 * @return Pointer to erased object, or NULL if invalid
 *
 * SAFETY:
 * - sexp must be a valid SEXP
 * - Must be called on R's main thread
 */
mx_erased *mx_get(SEXP sexp);

/**
 * Query an object for an interface vtable by tag.
 *
 * @param sexp R external pointer wrapping an erased object
 * @param tag Tag identifying the requested trait
 * @return Pointer to vtable if implemented, NULL otherwise
 *
 * SAFETY:
 * - sexp must be a valid SEXP
 * - Must be called on R's main thread
 * - Returned pointer must be cast to correct vtable type
 */
const void *mx_query(SEXP sexp, mx_tag tag);

/* ============================================================================
 * Registration (called from R_init_*)
 * ============================================================================ */

/**
 * Register the mx_* C-callables with R.
 *
 * Must be called from R_init_<pkg>() in the package.
 * Other packages load these via R_GetCCallable.
 */
void mx_abi_register(void);

#ifdef __cplusplus
}
#endif

#endif /* MINIEXTENDR_MX_ABI_H */
