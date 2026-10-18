# Third-party notices

TapWheel is free software under the GNU General Public License v3.0 or later (see LICENSE). It ships with the following work by others.

## FFmpeg

TapWheel runs `engines/ffmpeg.exe` as a separate program for images, audio and video.

- Version: 8.1.1, "essentials" build by Gyan Doshi (https://www.gyan.dev/ffmpeg/builds/)
- License: GNU General Public License v3.0 or later (built with `--enable-gpl --enable-version3`)
- Source of this version: https://ffmpeg.org/releases/ffmpeg-8.1.1.tar.xz
- Build recipe and the libraries it includes: https://github.com/GyanD/codexffmpeg/releases/tag/8.1.1

FFmpeg is a trademark of Fabrice Bellard, originator of the FFmpeg project.

## ONNX Runtime

TapWheel loads `engines/onnxruntime.dll` to run the Remove BG model on your PC.

- Version: 1.22.0 (https://github.com/microsoft/onnxruntime/releases/tag/v1.22.0)
- License: MIT, Copyright (c) Microsoft Corporation

## Background removal model

`engines/background.f16` holds the weights of IS-Net (general use), stored at half precision and expanded on first use.

- Model: IS-Net, "Highly Accurate Dichotomous Image Segmentation", Xuebin Qin and others (https://github.com/xuebinqin/DIS)
- ONNX export from rembg (https://github.com/danielgatis/rembg)
- License: Apache License 2.0

## Fluent UI System Icons

The wheel, cards and windows draw icons from Microsoft's Fluent UI System Icons (https://github.com/microsoft/fluentui-system-icons).

- License: MIT, Copyright (c) 2020 Microsoft Corporation

## JetBrains Mono

Code shots are set in JetBrains Mono (https://github.com/JetBrains/JetBrainsMono), shipped in `crates/core/assets`.

- License: SIL Open Font License 1.1, Copyright 2020 The JetBrains Mono Project Authors

## Syntax definitions

Code highlighting uses the syntax and theme collection bundled by the `two-face` crate, taken from the bat project (https://github.com/sharkdp/bat). Each definition keeps its own license (MIT, Apache-2.0, BSD or similar); see https://github.com/CosmicHorrorDev/two-face for the list.

## Rust crates

Compiled into `tapwheel.exe`. LGPL and MPL crates are used unmodified; their source is on crates.io at the version listed.

| Crate | Version | License |
|-------|---------|---------|
| ab_glyph | 0.2.32 | Apache-2.0 |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 |
| adler | 1.0.2 | 0BSD OR MIT OR Apache-2.0 |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| adler32 | 1.2.0 | Zlib |
| aes | 0.8.4 | MIT OR Apache-2.0 |
| aes | 0.9.3 | MIT OR Apache-2.0 |
| ahash | 0.7.8 | MIT OR Apache-2.0 |
| ahash | 0.8.12 | MIT OR Apache-2.0 |
| aho-corasick | 0.7.20 | Unlicense OR MIT |
| aho-corasick | 1.1.5 | Unlicense OR MIT |
| aligned | 0.4.3 | MIT OR Apache-2.0 |
| aligned-vec | 0.6.4 | MIT |
| alloc-no-stdlib | 2.0.4 | BSD-3-Clause |
| alloc-stdlib | 0.2.4 | BSD-3-Clause |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 |
| anyhow | 1.0.104 | MIT OR Apache-2.0 |
| ar_archive_writer | 0.5.3 | Apache-2.0 WITH LLVM-exception |
| arc-swap | 1.9.3 | MIT OR Apache-2.0 |
| arg_enum_proc_macro | 0.3.4 | MIT |
| arrayref | 0.3.9 | BSD-2-Clause |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| as-slice | 0.2.1 | MIT OR Apache-2.0 |
| ast_node | 5.0.0 | Apache-2.0 |
| atty | 0.2.14 | MIT |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| av-scenechange | 0.14.1 | MIT |
| av1-grain | 0.2.5 | BSD-2-Clause |
| avif-serialize | 0.8.9 | BSD-3-Clause |
| base64 | 0.22.1 | MIT OR Apache-2.0 |
| base64 | 0.23.1 | MIT OR Apache-2.0 |
| base64-simd | 0.7.0 | MIT |
| beef | 0.5.2 | MIT OR Apache-2.0 |
| better_scoped_tls | 1.0.1 | Apache-2.0 |
| bincode | 1.3.3 | MIT |
| bit-set | 0.8.0 | Apache-2.0 OR MIT |
| bit-vec | 0.6.3 | MIT/Apache-2.0 |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT |
| bit_field | 0.10.3 | Apache-2.0/MIT |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| bitstream-io | 4.10.0 | MIT/Apache-2.0 |
| bitvec | 1.1.1 | MIT |
| blake3 | 1.8.7 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 |
| block-padding | 0.3.3 | MIT OR Apache-2.0 |
| brotli-decompressor | 5.0.3 | BSD-3-Clause/MIT |
| bstr | 1.13.1 | MIT OR Apache-2.0 |
| built | 0.8.1 | MIT |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| bytecheck | 0.6.12 | MIT |
| bytecheck_derive | 0.6.12 | MIT |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| byteorder | 1.5.0 | Unlicense OR MIT |
| byteorder-lite | 0.1.0 | Unlicense OR MIT |
| bytes | 1.12.1 | MIT |
| bytes-str | 0.2.9 | Apache-2.0 |
| bzip2 | 0.6.1 | MIT OR Apache-2.0 |
| capacity_builder | 0.5.0 | MIT |
| capacity_builder_macros | 0.3.0 | MIT |
| cbc | 0.1.2 | MIT OR Apache-2.0 |
| cc | 1.8.0 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| chardetng | 0.1.17 | Apache-2.0 OR MIT |
| cipher | 0.4.4 | MIT OR Apache-2.0 |
| cipher | 0.5.2 | MIT OR Apache-2.0 |
| clap | 2.34.0 | MIT |
| cmov | 0.5.4 | Apache-2.0 OR MIT |
| color_quant | 1.1.0 | MIT |
| const-oid | 0.10.2 | Apache-2.0 OR MIT |
| const-str | 1.1.0 | MIT |
| constant_time_eq | 0.4.2 | CC0-1.0 OR MIT-0 OR Apache-2.0 |
| constant_time_eq | 0.6.1 | CC0-1.0 OR MIT-0 OR Apache-2.0 |
| convert_case | 0.6.0 | MIT |
| cookie | 0.18.2 | MIT OR Apache-2.0 |
| core_detect | 1.0.0 | MIT/Apache-2.0 |
| core_maths | 0.1.1 | MIT |
| countme | 3.0.1 | MIT OR Apache-2.0 |
| cpubits | 0.1.1 | MIT OR Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 |
| css_dataset | 0.4.1 | MIT |
| cssparser | 0.37.0 | MPL-2.0 |
| cssparser-color | 0.5.0 | MPL-2.0 |
| cssparser-macros | 0.7.1 | MPL-2.0 |
| csv | 1.4.0 | Unlicense/MIT |
| csv-core | 0.1.13 | Unlicense/MIT |
| ctutils | 0.4.3 | Apache-2.0 OR MIT |
| dashmap | 5.5.3 | MIT |
| data-encoding | 2.11.1 | MIT |
| data-url | 0.1.1 | MIT OR Apache-2.0 |
| data-url | 0.3.2 | MIT OR Apache-2.0 |
| deflate | 0.8.6 | MIT/Apache-2.0 |
| deno_ast | 0.53.3 | MIT |
| deno_error | 0.7.3 | MIT |
| deno_error_macro | 0.7.3 | MIT |
| deno_media_type | 0.4.0 | MIT |
| deno_terminal | 0.2.3 | MIT |
| deranged | 0.5.9 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| digest | 0.11.3 | MIT OR Apache-2.0 |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 |
| dpi | 0.1.2 | Apache-2.0 AND MIT |
| dprint-core | 0.67.4 | MIT |
| dprint-core-macros | 0.1.0 | MIT |
| dprint-plugin-typescript | 0.95.15 | MIT |
| dprint-swc-ext | 0.26.1 | MIT |
| dtoa | 1.0.11 | MIT OR Apache-2.0 |
| dtoa-short | 0.3.5 | MPL-2.0 |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 |
| either | 1.19.0 | MIT OR Apache-2.0 |
| embed-resource | 3.0.12 | MIT |
| encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| enumn | 0.1.14 | MIT OR Apache-2.0 |
| equator | 0.4.2 | MIT |
| equator-macro | 0.4.2 | MIT |
| equivalent | 1.0.3 | Apache-2.0 OR MIT |
| euclid | 0.22.14 | MIT OR Apache-2.0 |
| exr | 1.74.2 | BSD-3-Clause |
| fancy-regex | 0.16.2 | MIT |
| fastrand | 2.5.0 | Apache-2.0 OR MIT |
| fax | 0.2.7 | MIT |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| filetime | 0.2.29 | MIT/Apache-2.0 |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| flo_curves | 0.3.1 | Apache-2.0 |
| float-cmp | 0.9.0 | MIT |
| fnv | 1.0.7 | Apache-2.0 / MIT |
| foldhash | 0.1.5 | Zlib |
| fontdb | 0.23.0 | MIT |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 |
| from_variant | 3.0.0 | Apache-2.0 |
| funty | 2.0.0 | MIT |
| futf | 0.1.5 | MIT / Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| getrandom | 0.2.17 | MIT OR Apache-2.0 |
| getrandom | 0.3.4 | MIT OR Apache-2.0 |
| getrandom | 0.4.3 | MIT OR Apache-2.0 |
| gif | 0.11.4 | MIT/Apache-2.0 |
| gif | 0.13.3 | MIT OR Apache-2.0 |
| gif | 0.14.2 | MIT OR Apache-2.0 |
| glob | 0.3.4 | MIT OR Apache-2.0 |
| globset | 0.4.20 | Unlicense OR MIT |
| half | 2.7.1 | MIT OR Apache-2.0 |
| hashbrown | 0.12.3 | MIT OR Apache-2.0 |
| hashbrown | 0.13.2 | MIT OR Apache-2.0 |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| heck | 0.5.0 | MIT OR Apache-2.0 |
| hmac | 0.13.0 | MIT OR Apache-2.0 |
| hstr | 3.0.6 | Apache-2.0 |
| htmd | 0.3.2 | Apache-2.0 |
| html2text | 0.15.5 | MIT |
| html5ever | 0.35.0 | MIT OR Apache-2.0 |
| http | 1.5.0 | MIT OR Apache-2.0 |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 |
| icns | 0.3.1 | MIT |
| icu_collections | 2.3.0 | Unicode-3.0 |
| icu_locale_core | 2.3.0 | Unicode-3.0 |
| icu_normalizer | 2.3.0 | Unicode-3.0 |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 |
| icu_properties | 2.3.0 | Unicode-3.0 |
| icu_properties_data | 2.3.0 | Unicode-3.0 |
| icu_provider | 2.3.1 | Unicode-3.0 |
| idna | 1.1.0 | MIT OR Apache-2.0 |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT |
| image | 0.23.14 | MIT |
| image | 0.25.10 | MIT OR Apache-2.0 |
| image-webp | 0.2.4 | MIT OR Apache-2.0 |
| imagepipe | 0.5.1 | LGPL-3.0-only |
| imagequant | 4.4.1 | GPL-3.0-or-later |
| imagesize | 0.13.0 | MIT |
| imgref | 1.12.3 | CC0-1.0 OR Apache-2.0 |
| include_dir | 0.7.4 | MIT |
| include_dir_macros | 0.7.4 | MIT |
| indexmap | 1.9.3 | Apache-2.0 OR MIT |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| inout | 0.1.4 | MIT OR Apache-2.0 |
| inout | 0.2.4 | MIT OR Apache-2.0 |
| is-macro | 0.3.8 | Apache-2.0 |
| itertools | 0.10.5 | MIT/Apache-2.0 |
| itertools | 0.12.1 | MIT OR Apache-2.0 |
| itertools | 0.13.0 | MIT OR Apache-2.0 |
| itertools | 0.14.0 | MIT OR Apache-2.0 |
| itertools | 0.8.2 | MIT/Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| jobserver | 0.1.35 | MIT OR Apache-2.0 |
| jpeg-decoder | 0.1.22 | MIT / Apache-2.0 |
| jxl-bitstream | 1.1.0 | MIT OR Apache-2.0 |
| jxl-coding | 1.0.1 | MIT OR Apache-2.0 |
| jxl-color | 0.11.0 | MIT OR Apache-2.0 |
| jxl-frame | 0.13.3 | MIT OR Apache-2.0 |
| jxl-grid | 0.6.2 | MIT OR Apache-2.0 |
| jxl-image | 0.13.0 | MIT OR Apache-2.0 |
| jxl-jbr | 0.2.1 | MIT OR Apache-2.0 |
| jxl-modular | 0.11.3 | MIT OR Apache-2.0 |
| jxl-oxide | 0.12.6 | MIT OR Apache-2.0 |
| jxl-oxide-common | 1.0.0 | MIT OR Apache-2.0 |
| jxl-render | 0.12.4 | MIT OR Apache-2.0 |
| jxl-threadpool | 1.0.0 | MIT OR Apache-2.0 |
| jxl-vardct | 0.11.1 | MIT OR Apache-2.0 |
| kamadak-exif | 0.6.1 | BSD-2-Clause |
| kurbo | 0.11.3 | Apache-2.0 OR MIT |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| lebe | 0.5.3 | BSD-3-Clause |
| libbz2-rs-sys | 0.2.5 | bzip2-1.0.6 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| libm | 0.2.16 | MIT |
| lightningcss | 1.0.0-alpha.72 | MPL-2.0 |
| lightningcss-derive | 1.0.0-alpha.43 | MPL-2.0 |
| linked-hash-map | 0.5.6 | MIT/Apache-2.0 |
| litemap | 0.8.3 | Unicode-3.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| logos | 0.12.1 | MIT OR Apache-2.0 |
| logos-derive | 0.12.1 | MIT OR Apache-2.0 |
| loop9 | 0.1.5 | MIT |
| lzma-rust2 | 0.16.5 | Apache-2.0 |
| lzma-sys | 0.1.20 | MIT/Apache-2.0 |
| mac | 0.1.1 | MIT/Apache-2.0 |
| malva | 0.12.1 | MIT |
| markup5ever | 0.35.0 | MIT OR Apache-2.0 |
| markup5ever_rcdom | 0.35.0+unofficial | MIT OR Apache-2.0 |
| markup_fmt | 0.22.0 | MIT |
| match_token | 0.35.0 | MIT OR Apache-2.0 |
| matches | 0.1.10 | MIT |
| matrixmultiply | 0.3.11 | MIT/Apache-2.0 |
| maybe-rayon | 0.1.1 | MIT |
| md-5 | 0.10.6 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| memmap2 | 0.9.11 | MIT OR Apache-2.0 |
| memoffset | 0.9.1 | MIT |
| minify-html | 0.16.5 | MIT |
| minify-html-common | 0.0.2 | MIT |
| minify-js | 0.6.0 | Apache-2.0 |
| miniz_oxide | 0.3.7 | MIT |
| miniz_oxide | 0.4.4 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| multicache | 0.6.1 | LGPL-3.0-only |
| multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT |
| mutate_once | 0.1.2 | BSD-2-Clause |
| ndarray | 0.16.1 | MIT OR Apache-2.0 |
| new_debug_unreachable | 1.0.6 | MIT |
| no_std_io2 | 0.9.4 | Apache-2.0 OR MIT |
| nom | 8.0.0 | MIT |
| noop_proc_macro | 0.3.0 | MIT |
| num-bigint | 0.4.8 | MIT OR Apache-2.0 |
| num-complex | 0.4.6 | MIT OR Apache-2.0 |
| num-conv | 0.2.2 | MIT OR Apache-2.0 |
| num-derive | 0.4.2 | MIT OR Apache-2.0 |
| num-integer | 0.1.47 | MIT OR Apache-2.0 |
| num-iter | 0.1.46 | MIT OR Apache-2.0 |
| num-rational | 0.3.2 | MIT OR Apache-2.0 |
| num-rational | 0.4.2 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| object | 0.39.1 | Apache-2.0 OR MIT |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| ort | 2.0.0-rc.10 | MIT OR Apache-2.0 |
| ort-sys | 2.0.0-rc.10 | MIT OR Apache-2.0 |
| outref | 0.1.0 | MIT |
| owned_ttf_parser | 0.25.1 | Apache-2.0 |
| parcel_selectors | 0.28.3 | MPL-2.0 |
| parcel_sourcemap | 2.1.1 | MIT |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| parse-js | 0.20.1 | Apache-2.0 |
| paste | 1.0.15 | MIT OR Apache-2.0 |
| pastey | 0.1.1 | MIT OR Apache-2.0 |
| pathdiff | 0.2.3 | MIT/Apache-2.0 |
| pbkdf2 | 0.13.0 | MIT OR Apache-2.0 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| phf | 0.11.3 | MIT |
| phf | 0.13.1 | MIT |
| phf_codegen | 0.11.3 | MIT |
| phf_generator | 0.11.3 | MIT |
| phf_generator | 0.13.1 | MIT |
| phf_macros | 0.11.3 | MIT |
| phf_macros | 0.13.1 | MIT |
| phf_shared | 0.11.3 | MIT |
| phf_shared | 0.13.1 | MIT |
| pico-args | 0.5.0 | MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| plist | 1.10.1 | MIT |
| png | 0.16.8 | MIT OR Apache-2.0 |
| png | 0.17.16 | MIT OR Apache-2.0 |
| png | 0.18.1 | MIT OR Apache-2.0 |
| potential_utf | 0.1.6 | Unicode-3.0 |
| powerfmt | 0.2.1 | MIT OR Apache-2.0 |
| ppmd-rust | 1.5.0 | CC0-1.0 OR MIT-0 |
| precomputed-hash | 0.1.1 | MIT |
| pretty_yaml | 0.5.1 | MIT |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| profiling | 1.0.18 | MIT OR Apache-2.0 |
| profiling-procmacros | 1.0.18 | MIT OR Apache-2.0 |
| psd | 0.3.5 | MIT/Apache-2.0 |
| psm | 0.1.32 | MIT OR Apache-2.0 |
| ptr_meta | 0.1.4 | MIT |
| ptr_meta_derive | 0.1.4 | MIT |
| pulldown-cmark | 0.13.4 | MIT |
| pulldown-cmark-escape | 0.11.0 | MIT |
| pulp | 0.22.3 | MIT |
| pulp-wasm-simd-flag | 0.1.1 | MIT |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 |
| qoi | 0.4.1 | MIT/Apache-2.0 |
| quick-error | 2.0.1 | MIT/Apache-2.0 |
| quick-xml | 0.38.4 | MIT |
| quick-xml | 0.42.0 | MIT |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| radium | 0.7.0 | MIT |
| raffia | 0.9.2 | MIT |
| raffia_macro | 0.1.1 | MIT |
| rand | 0.8.8 | MIT OR Apache-2.0 |
| rand_core | 0.6.4 | MIT OR Apache-2.0 |
| rav1e | 0.8.1 | BSD-2-Clause |
| ravif | 0.13.0 | BSD-3-Clause |
| raw-cpuid | 11.6.0 | MIT |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| rawloader | 0.37.2 | LGPL-2.1 |
| rawpointer | 0.2.1 | MIT/Apache-2.0 |
| rayon | 1.12.0 | MIT OR Apache-2.0 |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 |
| reborrow | 0.5.5 | MIT |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 |
| regex-syntax | 0.6.29 | MIT OR Apache-2.0 |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 |
| rend | 0.4.2 | MIT |
| resvg | 0.45.1 | Apache-2.0 OR MIT |
| rgb | 0.8.53 | MIT |
| rkyv | 0.7.46 | MIT |
| rkyv_derive | 0.7.46 | MIT |
| roots | 0.0.6 | BSD-2-Clause |
| rowan | 0.15.19 | MIT OR Apache-2.0 |
| rowan | 0.16.1 | MIT OR Apache-2.0 |
| roxmltree | 0.20.0 | MIT OR Apache-2.0 |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| rustybuzz | 0.20.1 | MIT |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 |
| same-file | 1.0.6 | Unlicense/MIT |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 |
| scoped_threadpool | 0.1.9 | MIT |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| seahash | 4.1.0 | MIT |
| semver | 1.0.28 | MIT OR Apache-2.0 |
| seq-macro | 0.3.6 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde-content | 0.1.2 | MIT/Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| serde_spanned | 1.1.2 | MIT OR Apache-2.0 |
| serde_yaml | 0.8.26 | MIT OR Apache-2.0 |
| serde_yaml | 0.9.34+deprecated | MIT OR Apache-2.0 |
| sevenz-rust2 | 0.20.2 | Apache-2.0 |
| sha1 | 0.10.7 | MIT OR Apache-2.0 |
| sha1 | 0.11.0 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| simd-abstraction | 0.7.1 | MIT |
| simd-adler32 | 0.3.10 | MIT |
| simd_helpers | 0.1.0 | MIT |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| simplecss | 0.2.2 | Apache-2.0 OR MIT |
| siphasher | 0.3.11 | MIT/Apache-2.0 |
| siphasher | 1.0.4 | MIT OR Apache-2.0 |
| slotmap | 1.1.1 | Zlib |
| smallvec | 1.16.3 | MIT OR Apache-2.0 |
| smallvec | 2.0.0-alpha.10 | MIT OR Apache-2.0 |
| smartstring | 1.0.1 | MPL-2.0+ |
| sqlformat | 0.3.5 | MIT OR Apache-2.0 |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 |
| stacker | 0.1.25 | MIT OR Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| strict-num | 0.1.1 | MIT |
| string_cache | 0.8.9 | MIT OR Apache-2.0 |
| string_cache_codegen | 0.5.4 | MIT OR Apache-2.0 |
| string_enum | 1.0.2 | Apache-2.0 |
| strsim | 0.8.0 | MIT |
| svgtypes | 0.15.3 | Apache-2.0 OR MIT |
| swc_atoms | 9.0.0 | Apache-2.0 |
| swc_common | 17.0.1 | Apache-2.0 |
| swc_ecma_ast | 18.0.0 | Apache-2.0 |
| swc_ecma_lexer | 26.0.0 | Apache-2.0 |
| swc_ecma_parser | 27.0.7 | Apache-2.0 |
| swc_eq_ignore_macros | 1.0.1 | Apache-2.0 |
| swc_macros_common | 1.0.1 | Apache-2.0 |
| swc_visit | 2.0.2 | Apache-2.0 |
| syn | 1.0.109 | MIT OR Apache-2.0 |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| synstructure | 0.14.0 | MIT |
| syntect | 5.3.0 | MIT |
| tap | 1.0.1 | MIT |
| taplo | 0.14.0 | MIT |
| tar | 0.4.46 | MIT OR Apache-2.0 |
| tendril | 0.4.3 | MIT/Apache-2.0 |
| termcolor | 1.4.1 | Unlicense OR MIT |
| text-size | 1.1.1 | MIT OR Apache-2.0 |
| text_lines | 0.6.0 | MIT |
| textwrap | 0.11.0 | MIT |
| thiserror | 1.0.69 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |
| thread_local | 1.1.10 | MIT OR Apache-2.0 |
| tiff | 0.11.3 | MIT |
| tiff | 0.6.1 | MIT |
| time | 0.3.55 | MIT OR Apache-2.0 |
| time-core | 0.1.9 | MIT OR Apache-2.0 |
| time-macros | 0.2.32 | MIT OR Apache-2.0 |
| tiny-skia | 0.11.4 | BSD-3-Clause |
| tiny-skia | 0.12.0 | BSD-3-Clause |
| tiny-skia-path | 0.11.4 | BSD-3-Clause |
| tiny-skia-path | 0.12.0 | BSD-3-Clause |
| tiny_pretty | 0.2.1 | MIT |
| tinystr | 0.8.4 | Unicode-3.0 |
| tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT |
| toml | 0.5.11 | MIT/Apache-2.0 |
| toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_datetime | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_writer | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 |
| tracing | 0.1.44 | MIT |
| tracing-attributes | 0.1.31 | MIT |
| tracing-core | 0.1.36 | MIT |
| triomphe | 0.1.17 | MIT OR Apache-2.0 |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 |
| two-face | 0.4.5 | MIT OR Apache-2.0 |
| typed-path | 0.12.3 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicase | 2.10.0 | MIT OR Apache-2.0 |
| unicode-bidi | 0.3.18 | MIT OR Apache-2.0 |
| unicode-bidi-mirroring | 0.4.0 | MIT/Apache-2.0 |
| unicode-ccc | 0.4.0 | MIT/Apache-2.0 |
| unicode-id-start | 1.5.0 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-properties | 0.1.4 | MIT/Apache-2.0 |
| unicode-script | 0.5.8 | MIT OR Apache-2.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| unicode-vo | 0.1.0 | MIT/Apache-2.0 |
| unicode-width | 0.1.14 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| unicode_categories | 0.1.1 | MIT OR Apache-2.0 |
| unsafe-libyaml | 0.2.11 | MIT |
| url | 2.5.8 | MIT OR Apache-2.0 |
| usvg | 0.45.1 | Apache-2.0 OR MIT |
| utf-8 | 0.7.6 | MIT OR Apache-2.0 |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT |
| uuid | 1.28.0 | Apache-2.0 OR MIT |
| v_frame | 0.3.9 | BSD-2-Clause |
| vec_map | 0.8.2 | MIT/Apache-2.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| visioncortex | 0.8.10 | MIT OR Apache-2.0 |
| vlq | 0.5.1 | Apache-2.0/MIT |
| vswhom | 0.1.0 | MIT |
| vswhom-sys | 0.1.3 | MIT |
| vtracer | 0.6.5 | MIT OR Apache-2.0 |
| walkdir | 2.5.0 | Unlicense/MIT |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 |
| web_atoms | 0.1.3 | MIT OR Apache-2.0 |
| webview2-com | 0.39.1 | MIT |
| webview2-com-macros | 0.8.1 | MIT |
| webview2-com-sys | 0.39.1 | MIT |
| weezl | 0.1.12 | MIT OR Apache-2.0 |
| winapi | 0.3.9 | MIT/Apache-2.0 |
| winapi-util | 0.1.11 | Unlicense OR MIT |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| windows-version | 0.1.7 | MIT OR Apache-2.0 |
| winnow | 0.6.26 | MIT |
| winnow | 0.7.15 | MIT |
| winnow | 1.0.4 | MIT |
| winreg | 0.56.0 | MIT |
| writeable | 0.6.4 | Unicode-3.0 |
| wry | 0.57.0 | Apache-2.0 OR MIT |
| wyz | 0.5.1 | MIT |
| xml5ever | 0.35.0 | MIT OR Apache-2.0 |
| xmlwriter | 0.1.0 | MIT |
| xz2 | 0.1.7 | MIT/Apache-2.0 |
| y4m | 0.8.0 | MIT |
| yaml-rust | 0.4.5 | MIT/Apache-2.0 |
| yaml_parser | 0.2.3 | MIT |
| yoke | 0.8.3 | Unicode-3.0 |
| yoke-derive | 0.8.4 | Unicode-3.0 |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerofrom | 0.1.8 | Unicode-3.0 |
| zerofrom-derive | 0.1.8 | Unicode-3.0 |
| zeroize | 1.9.1 | Apache-2.0 OR MIT |
| zerotrie | 0.2.5 | Unicode-3.0 |
| zerovec | 0.11.8 | Unicode-3.0 |
| zerovec-derive | 0.11.6 | Unicode-3.0 |
| zip | 9.0.5 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |
| zopfli | 0.8.3 | Apache-2.0 |
| zstd | 0.13.3 | MIT |
| zstd-safe | 7.3.0 | BSD-3-Clause |
| zstd-sys | 2.1.2+zstd.1.5.7 | BSD-3-Clause |
| zune-core | 0.4.12 | MIT OR Apache-2.0 OR Zlib |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib |
| zune-inflate | 0.2.54 | MIT OR Apache-2.0 OR Zlib |
| zune-jpeg | 0.4.21 | MIT OR Apache-2.0 OR Zlib |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib |
