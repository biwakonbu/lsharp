fn emit_component_cli_readlink_file_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.readlink-at" (func $readlink-at (type 4)))
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
  (data (i32.const 128) "link.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
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
      i32.const 128
      i32.const 8
      i32.const 32
      call $readlink-at
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
      i32.const 40
      i32.load
      call $stdout-write
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("readlink-at probe module を生成できる")
}

fn emit_component_cli_link_file_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.link-at" (func $link-at (type 3)))
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
  (data (i32.const 128) "source.txt")
  (data (i32.const 160) "hardlink.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
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
      local.get $preopen
      i32.const 160
      i32.const 12
      i32.const 32
      call $link-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("link-at probe module を生成できる")
}

fn emit_component_cli_same_object_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32) (result i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.is-same-object" (func $is-same-object (type 4)))
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
  (data (i32.const 128) "source.txt")
  (data (i32.const 160) "hardlink.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $left i32)
    (local $right i32)
    (local $same i32)
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
      i32.const 0
      i32.const 0
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if (result i32)
        local.get $preopen
        call $drop-descriptor
        i32.const 1
      else
        i32.const 36
        i32.load
        local.set $left
        local.get $preopen
        i32.const 0
        i32.const 160
        i32.const 12
        i32.const 0
        i32.const 0
        i32.const 40
        call $open-at
        i32.const 40
        i32.load8_u
        if (result i32)
          local.get $left
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          i32.const 44
          i32.load
          local.set $right
          local.get $left
          local.get $right
          call $is-same-object
          local.set $same
          local.get $right
          call $drop-descriptor
          local.get $left
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          local.get $same
          i32.eqz
        end
      end
    end)
)
"#,
    )
    .expect("is-same-object probe module を生成できる")
}

fn emit_component_cli_metadata_hash_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.metadata-hash" (func $metadata-hash (type 0)))
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
  (data (i32.const 128) "source.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $same i32)
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
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if (result i32)
        local.get $preopen
        call $drop-descriptor
        i32.const 1
      else
        i32.const 36
        i32.load
        local.set $descriptor
        local.get $descriptor
        i32.const 64
        call $metadata-hash
        i32.const 64
        i32.load8_u
        if (result i32)
          local.get $descriptor
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          local.get $descriptor
          i32.const 96
          call $metadata-hash
          i32.const 96
          i32.load8_u
          if (result i32)
            local.get $descriptor
            call $drop-descriptor
            local.get $preopen
            call $drop-descriptor
            i32.const 1
          else
            i32.const 72
            i64.load
            i32.const 104
            i64.load
            i64.ne
            if (result i32)
              i32.const 1
            else
              i32.const 80
              i64.load
              i32.const 112
              i64.load
              i64.ne
            end
            local.set $same
            local.get $descriptor
            call $drop-descriptor
            local.get $preopen
            call $drop-descriptor
            local.get $same
          end
        end
      end
    end)
)
"#,
    )
    .expect("metadata-hash probe module を生成できる")
}

fn emit_component_cli_metadata_hash_at_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.metadata-hash-at" (func $metadata-hash-at (type 4)))
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
  (data (i32.const 128) "source.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $same i32)
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
      i32.const 64
      call $metadata-hash-at
      i32.const 64
      i32.load8_u
      if (result i32)
        local.get $preopen
        call $drop-descriptor
        i32.const 1
      else
        local.get $preopen
        i32.const 0
        i32.const 128
        i32.const 10
        i32.const 96
        call $metadata-hash-at
        i32.const 96
        i32.load8_u
        if (result i32)
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          i32.const 72
          i64.load
          i32.const 104
          i64.load
          i64.ne
          if (result i32)
            i32.const 1
          else
            i32.const 80
            i64.load
            i32.const 112
            i64.load
            i64.ne
          end
          local.set $same
          local.get $preopen
          call $drop-descriptor
          local.get $same
        end
      end
    end)
)
"#,
    )
    .expect("metadata-hash-at probe module を生成できる")
}

fn emit_component_cli_stat_at_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.stat-at" (func $stat-at (type 2)))
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
  (data (i32.const 128) "source.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    (local $preopen i32)
    (local $ok i32)
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
      i32.const 64
      call $stat-at
      i32.const 64
      i32.load8_u
      if (result i32)
        local.get $preopen
        call $drop-descriptor
        i32.const 1
      else
        i32.const 72
        i32.load8_u
        i32.const 6
        i32.ne
        if (result i32)
          i32.const 1
        else
          i32.const 88
          i64.load
          i64.const 5
          i64.ne
        end
        local.set $ok
        local.get $preopen
        call $drop-descriptor
        local.get $ok
      end
    end)
)
"#,
    )
    .expect("stat-at probe module を生成できる")
}

fn emit_component_cli_set_times_at_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i64 i32 i32 i64 i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.set-times-at" (func $set-times-at (type 2)))
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
  (data (i32.const 128) "source.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    (local $preopen i32)
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
      i32.const 0
      i64.const 0
      i32.const 0
      i32.const 0
      i64.const 0
      i32.const 0
      i32.const 64
      call $set-times-at
      i32.const 64
      i32.load8_u
      if (result i32)
        local.get $preopen
        call $drop-descriptor
        i32.const 1
      else
        local.get $preopen
        call $drop-descriptor
        i32.const 0
      end
    end)
)
"#,
    )
    .expect("set-times-at probe module を生成できる")
}

fn emit_component_cli_poll_list_probe_module() -> Vec<u8> {
    emit_component_cli_poll_list_probe_module_with_list_len(1)
}
