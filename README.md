bouffalo-cli
============

bouffalo-cli is a command-line interface utility to interact with the bl602 boot
rom

## Features

| Feature                           | Supported |
|-----------------------------------|-----------|
| Boot rom info                     | ✅        |
| Converting elf to firmware image  | ❎        |

| Medium                            | Read | Write | Erase | Verify |
|-----------------------------------|------|-------|-------|--------|
| Flash                             | ❎   | ❎    | ❎    | ❎     |
| RAM                               | ❎   | ❎    | ❎    | ❎     |

## Examples

### Getting BootROM info

If you want to get the BootROM version and the OTP flags, you can run the
following command:

```
% bouffalo-cli info
```

And it should print something similar to this (output is from a new DT-BL10
board):

```
Using serial device "/dev/ttyUSB0"
BootROM version: 1
OTP flags:
  00000000 00000000 00000000 00000000
  00000011 00000000 00000000 00000000
  01011000 10011110 00000010 01000010
  11101000 10110100 00011101 00000000
```
