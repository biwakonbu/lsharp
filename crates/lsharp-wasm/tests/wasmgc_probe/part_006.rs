fn emit_component_cli_direct_write_stat_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i64 i32)))
  (type (func (param i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write" (func $write (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.stat" (func $stat (type 5)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "output.txt")
  (data (i32.const 256) "hello")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 10
      i32.const 5
      i32.const 2
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 256
      i32.const 5
      i64.const 0
      i32.const 40
      call $write
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 48
      i64.load
      i64.const 5
      i64.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      i32.const 64
      call $stat
      i32.const 64
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 72
      i32.load
      i32.const 6
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 88
      i64.load
      i64.const 5
      i64.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor direct write/stat probe module を生成できる")
}

fn emit_component_cli_direct_write_error_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write" (func $write (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (data (i32.const 256) "!")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 256
      i32.const 1
      i64.const 0
      i32.const 40
      call $write
      i32.const 40
      i32.load8_u
      i32.eqz
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor direct write error probe module を生成できる")
}

fn emit_component_cli_descriptor_type_flags_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.get-type" (func $get-type (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.get-flags" (func $get-flags (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 40
      call $get-type
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 41
      i32.load8_u
      i32.const 6
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      i32.const 48
      call $get-flags
      i32.const 48
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 49
      i32.load8_u
      i32.const 1
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor type/flags probe module を生成できる")
}

fn emit_component_cli_pollable_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32) (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.subscribe" (func $subscribe (type 5)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.block" (func $block (type 1)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.ready" (func $ready (type 5)))
  (import "wasi:io/poll@0.2.3" "[resource-drop]pollable" (func $drop-pollable (param i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]input-stream" (func $drop-input-stream (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (data (i32.const 144) "R")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $pollable i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i64.const 0
      i32.const 40
      call $read-via-stream
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      local.set $stream
      local.get $stream
      call $subscribe
      local.set $pollable
      local.get $pollable
      call $block
      local.get $pollable
      call $ready
      i32.eqz
      if
        local.get $pollable
        call $drop-pollable
        local.get $stream
        call $drop-input-stream
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 144
      i32.const 1
      call $stdout-write
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-input-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("pollable probe module を生成できる")
}

fn emit_component_cli_sync_data_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.sync-data" (func $sync-data (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 40
      call $sync-data
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("sync-data probe module を生成できる")
}

fn emit_component_cli_sync_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.sync" (func $sync (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 40
      call $sync
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("sync probe module を生成できる")
}
