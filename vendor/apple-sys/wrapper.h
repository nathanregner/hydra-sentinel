@import CoreFoundation;
@import IOKit;

// Re-export macros as static consts so bindgen can evaluate them. 
static const uint32_t IOMessageCanSystemSleep     = kIOMessageCanSystemSleep;
static const uint32_t IOMessageSystemWillSleep    = kIOMessageSystemWillSleep;
static const uint32_t IOMessageSystemWillNotSleep = kIOMessageSystemWillNotSleep;
static const uint32_t IOMessageSystemHasPoweredOn = kIOMessageSystemHasPoweredOn;
static const uint32_t IOMessageSystemWillPowerOn  = kIOMessageSystemWillPowerOn;
