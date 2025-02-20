#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

/// The number of Unicode Scalar Values.
constexpr static const uintptr_t NUMBER_USVS = 1114112;

/// The number of basic TeX register.
constexpr static const uintptr_t NUMBER_REGS = 256;

/// The number of TeX math fonts.
constexpr static const uintptr_t NUMBER_MATH_FONTS = (3 * 256);

/// The number of bytes in a TeX "memory word" variable.
constexpr static const uintptr_t SIZEOF_MEMORY_WORD = 8;

/// The minimum allowed value of a TeX "halfword" variable
constexpr static const int32_t MIN_HALFWORD = -268435455;

/// The maximum allowed value of a TeX "halfword" variable
constexpr static const int32_t MAX_HALFWORD = 1073741823;

/// The value of a "null" memory pointer in TeX.
constexpr static const int32_t TEX_NULL = MIN_HALFWORD;

/// A type for format file version numbers.
///
/// This is `usize`. Version numbers increment monotonically as engine commands
/// and primitives evolve
using FormatVersion = uintptr_t;

/// The latest format version number supported by this version of the crate.
constexpr static const FormatVersion LATEST_VERSION = 34;
