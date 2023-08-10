# Zenoh-hammer

Zenoh ui tool.   
Convenient for simple zenoh network communication testing.

The functionality provided is similar to the zenoh command line tools z_sub, z_put, z_get.

[中文/chinese readme](https://github.com/sanri/zenoh-hammer/blob/main/README.zh.md)


## Example

<img src="media/example.gif">


## Features
- [x] Support sending, receiving, and viewing text type data.
- [x] Support sending, receiving, and viewing image data in png and jpeg formats.
- [x] Message content can be viewed with a hexadecimal viewer,currently only the first 5KB of the message can be viewed.
    - [ ] Hex viewer supports viewing data within 100MB.
- [x] The configuration data in the software interface can be saved as a file.
- [x] Counts how often subscription data is received.
- [x] Support Chinese and English.
    - [ ] The interface display language supports runtime switching.
- [ ] Ability to load zenoh communication configuration files.


## Build

run the command in the project home directory

```shell
cargo build --release
```
