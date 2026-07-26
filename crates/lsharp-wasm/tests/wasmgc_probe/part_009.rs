fn emit_component_cli_poll_list_probe_module_with_list_len(list_len: u32) -> Vec<u8> {
    let wat = r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.subscribe" (func $subscribe (type 5)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.block" (func $block (type 1)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.ready" (func $ready (type 5)))
  (import "wasi:io/poll@0.2.3" "poll" (func $poll (type 6)))
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
  (data (i32.const 144) "P")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $pollable i32)
    (local $pollable2 i32)
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
      local.get $stream
      call $subscribe
      local.set $pollable2
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
      local.get $pollable2
      call $block
      local.get $pollable2
      call $ready
      i32.eqz
      if
        local.get $pollable2
        call $drop-pollable
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
      i32.const 64
      local.get $pollable
      i32.store
      i32.const 68
      local.get $pollable2
      i32.store
      i32.const 64
      i32.const __POLL_LIST_LEN__
      i32.const 72
      call $poll
      i32.const 76
      i32.load
      i32.const __POLL_LIST_LEN__
      i32.ne
      if
        local.get $pollable2
        call $drop-pollable
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
      i32.const 72
      i32.load
      i32.load
      i32.const 0
      i32.ne
      if
        local.get $pollable2
        call $drop-pollable
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
      i32.const __CHECK_SECOND__
      if
        i32.const 72
        i32.load
        i32.const 4
        i32.add
        i32.load
        i32.const 1
        i32.ne
        if
          local.get $pollable2
          call $drop-pollable
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
      end
      i32.const 144
      i32.const 1
      call $stdout-write
      local.get $pollable2
      call $drop-pollable
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
"#
    .replace("__POLL_LIST_LEN__", &list_len.to_string())
    .replace("__CHECK_SECOND__", if list_len == 2 { "1" } else { "0" });
    wat::parse_str(wat).expect("poll list probe module を生成できる")
}

fn emit_component_cli_poll_list_probe_module_from_two_input_streams() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.subscribe" (func $subscribe (type 5)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.block" (func $block (type 1)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.ready" (func $ready (type 5)))
  (import "wasi:io/poll@0.2.3" "poll" (func $poll (type 6)))
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
  (data (i32.const 128) "source-a.txt")
  (data (i32.const 144) "P")
  (data (i32.const 176) "source-b.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $descriptor2 i32)
    (local $stream i32)
    (local $stream2 i32)
    (local $pollable i32)
    (local $pollable2 i32)
    (local $index0 i32)
    (local $index1 i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 1
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
      i32.const 12
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
      local.get $preopen
      i32.const 0
      i32.const 176
      i32.const 12
      i32.const 0
      i32.const 1
      i32.const 40
      call $open-at
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
      local.set $descriptor2
      local.get $descriptor
      i64.const 0
      i32.const 48
      call $read-via-stream
      i32.const 48
      i32.load8_u
      if
        local.get $descriptor2
        call $drop-descriptor
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 52
      i32.load
      local.set $stream
      local.get $descriptor2
      i64.const 0
      i32.const 56
      call $read-via-stream
      i32.const 56
      i32.load8_u
      if
        local.get $stream
        call $drop-input-stream
        local.get $descriptor2
        call $drop-descriptor
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 60
      i32.load
      local.set $stream2
      local.get $stream
      call $subscribe
      local.set $pollable
      local.get $stream2
      call $subscribe
      local.set $pollable2
      local.get $pollable
      call $block
      local.get $pollable
      call $ready
      i32.eqz
      if
        local.get $pollable2
        call $drop-pollable
        local.get $pollable
        call $drop-pollable
        local.get $stream2
        call $drop-input-stream
        local.get $stream
        call $drop-input-stream
        local.get $descriptor2
        call $drop-descriptor
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $pollable2
      call $block
      local.get $pollable2
      call $ready
      i32.eqz
      if
        local.get $pollable2
        call $drop-pollable
        local.get $pollable
        call $drop-pollable
        local.get $stream2
        call $drop-input-stream
        local.get $stream
        call $drop-input-stream
        local.get $descriptor2
        call $drop-descriptor
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 64
      local.get $pollable
      i32.store
      i32.const 68
      local.get $pollable2
      i32.store
      i32.const 64
      i32.const 2
      i32.const 72
      call $poll
      i32.const 76
      i32.load
      i32.const 2
      i32.ne
      if
        local.get $pollable2
        call $drop-pollable
        local.get $pollable
        call $drop-pollable
        local.get $stream2
        call $drop-input-stream
        local.get $stream
        call $drop-input-stream
        local.get $descriptor2
        call $drop-descriptor
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 72
      i32.load
      i32.load
      local.set $index0
      i32.const 72
      i32.load
      i32.const 4
      i32.add
      i32.load
      local.set $index1
      local.get $index0
      i32.const 2
      i32.lt_u
      local.get $index1
      i32.const 2
      i32.lt_u
      i32.and
      local.get $index0
      local.get $index1
      i32.add
      i32.const 1
      i32.eq
      i32.and
      i32.eqz
      if
        local.get $pollable2
        call $drop-pollable
        local.get $pollable
        call $drop-pollable
        local.get $stream2
        call $drop-input-stream
        local.get $stream
        call $drop-input-stream
        local.get $descriptor2
        call $drop-descriptor
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $pollable2
      call $drop-pollable
      local.get $pollable
      call $drop-pollable
      local.get $stream2
      call $drop-input-stream
      local.get $stream
      call $drop-input-stream
      local.get $descriptor2
      call $drop-descriptor
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 144
      i32.const 1
      call $stdout-write
      i32.const 0
    end)
)
"#,
    )
    .expect("multiple input source poll list probe module を生成できる")
}

fn emit_component_cli_read_directory_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-directory" (func $read-directory (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]directory-entry-stream.read-directory-entry" (func $read-directory-entry (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]directory-entry-stream" (func $drop-directory-entry-stream (param i32)))
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
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $stream i32)
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
      i32.const 24
      call $read-directory
      i32.const 24
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 28
      i32.load
      local.set $stream
      local.get $stream
      i32.const 32
      call $read-directory-entry
      i32.const 32
      i32.load8_u
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load8_u
      i32.const 1
      i32.ne
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 40
      i32.load
      i32.const 6
      i32.ne
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      i32.const 48
      i32.load
      call $stdout-write
      local.get $stream
      i32.const 64
      call $read-directory-entry
      i32.const 64
      i32.load8_u
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 68
      i32.load8_u
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $stream
      call $drop-directory-entry-stream
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("read-directory probe module を生成できる")
}
