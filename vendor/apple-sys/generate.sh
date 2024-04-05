bindgen \
 --no-include-path-detection \
 --allowlist-function IOPMAssertionCreateWithName \
 --allowlist-function IOPMAssertionRelease \
 --allowlist-var kIOReturnSuccess \
 --allowlist-var kIOPMAssertionLevelOn \
 wrapper.h \
 -- \
 -x objective-c -fblocks -fmodules \
 -isysroot $(xcrun --sdk macosx --show-sdk-path) \
> src/IOKit.rs

