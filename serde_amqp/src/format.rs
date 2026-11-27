/// Offset of List in other implementations
/// The two implementations below do not count the byte(s) taken by `size` itself
/// 1. amqpnetlite: for List32
/// ```csharp
/// listSize = (totalBufferLength - 5); // totalBufferLength includes format code
/// ```
/// 2. qpid-protonj2: for List32
/// ```java
/// buffer.setInt(startIndex, endIndex - startIndex - 4); // startIndex is the byte after format code, endIndex is the end of the buffer
/// ```
/// offset includes 1 byte of `count`
pub const OFFSET_LIST8: usize = 1;
/// offset includes 4 byte of `count`
pub const OFFSET_LIST32: usize = 4;

/// offset includes 1 byte of `count`
pub const OFFSET_MAP8: usize = 1;
/// offset includes 4 byte of `count`
pub const OFFSET_MAP32: usize = 4;

/// offset includes 1 byte of `count` and 1 byte of element format code
pub const OFFSET_ARRAY8: usize = 2;
/// offset includes 4 bytes of `count` and 1 byte of element format code
pub const OFFSET_ARRAY32: usize = 5;
