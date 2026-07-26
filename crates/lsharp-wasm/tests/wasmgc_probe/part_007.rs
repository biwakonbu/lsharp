fn emit_component_cli_set_size_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.set-size" (func $set-size (type 4)))
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
      i64.const 7
      i32.const 40
      call $set-size
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
    .expect("set-size probe module を生成できる")
}

fn emit_component_cli_set_times_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i64 i32 i32 i64 i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 2)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.set-times" (func $set-times (type 3)))
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
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
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
      i32.const 2
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
        i32.const 0
        i64.const 0
        i32.const 0
        i32.const 0
        i64.const 0
        i32.const 0
        i32.const 64
        call $set-times
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
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 0
        end
      end
    end)
)
"#,
    )
    .expect("set-times probe module を生成できる")
}

fn emit_component_cli_advise_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i64 i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 2)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.advise" (func $advise (type 3)))
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
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
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
      if (result i32)
        local.get $preopen
        call $drop-descriptor
        i32.const 1
      else
        i32.const 36
        i32.load
        local.set $descriptor
        local.get $descriptor
        i64.const 0
        i64.const 5
        i32.const 0
        i32.const 64
        call $advise
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
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 0
        end
      end
    end)
)
"#,
    )
    .expect("advise probe module を生成できる")
}

fn emit_component_cli_create_directory_probe_module() -> Vec<u8> {
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
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.create-directory-at" (func $create-directory-at (type 4)))
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
  (data (i32.const 128) "created")
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
      i32.const 7
      i32.const 32
      call $create-directory-at
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
    .expect("create-directory-at probe module を生成できる")
}

fn emit_component_cli_remove_directory_probe_module() -> Vec<u8> {
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
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.remove-directory-at" (func $remove-directory-at (type 4)))
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
  (data (i32.const 128) "to-remove")
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
      i32.const 9
      i32.const 32
      call $remove-directory-at
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
    .expect("remove-directory-at probe module を生成できる")
}

fn emit_component_cli_unlink_file_probe_module() -> Vec<u8> {
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
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.unlink-file-at" (func $unlink-file-at (type 4)))
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
  (data (i32.const 128) "to-unlink.txt")
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
      i32.const 13
      i32.const 32
      call $unlink-file-at
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
    .expect("unlink-file-at probe module を生成できる")
}

fn emit_component_cli_rename_file_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.rename-at" (func $rename-at (type 5)))
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
  (data (i32.const 128) "old.txt")
  (data (i32.const 160) "renamed.txt")
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
      i32.const 7
      local.get $preopen
      i32.const 160
      i32.const 11
      i32.const 32
      call $rename-at
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
    .expect("rename-at probe module を生成できる")
}

fn emit_component_cli_symlink_file_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.symlink-at" (func $symlink-at (type 5)))
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
  (data (i32.const 128) "target.txt")
  (data (i32.const 160) "link.txt")
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
      i32.const 10
      i32.const 160
      i32.const 8
      i32.const 32
      call $symlink-at
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
    .expect("symlink-at probe module を生成できる")
}
