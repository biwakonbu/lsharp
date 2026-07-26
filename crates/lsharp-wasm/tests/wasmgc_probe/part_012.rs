fn emit_component_cli_skip_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.skip" (func $skip (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.blocking-skip" (func $blocking-skip (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.blocking-read" (func $blocking-read (type 4)))
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
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $remaining i64)
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
        i32.const 40
        call $read-via-stream
        i32.const 40
        i32.load8_u
        if (result i32)
          local.get $descriptor
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          i32.const 44
          i32.load
          local.set $stream
          local.get $stream
          i64.const 2
          i32.const 48
          call $skip
          i32.const 48
          i32.load8_u
          if (result i32)
            local.get $stream
            call $drop-input-stream
            local.get $descriptor
            call $drop-descriptor
            local.get $preopen
            call $drop-descriptor
            i32.const 1
          else
            i32.const 56
            i64.load
            i64.const 2
            i64.gt_u
            if (result i32)
              local.get $stream
              call $drop-input-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 1
            else
              i64.const 2
              i32.const 56
              i64.load
              i64.sub
              local.set $remaining
              local.get $stream
              local.get $remaining
              i32.const 64
              call $blocking-skip
              i32.const 64
              i32.load8_u
              if (result i32)
                local.get $stream
                call $drop-input-stream
                local.get $descriptor
                call $drop-descriptor
                local.get $preopen
                call $drop-descriptor
                i32.const 1
              else
                i32.const 72
                i64.load
                local.get $remaining
                i64.ne
                if (result i32)
                  local.get $stream
                  call $drop-input-stream
                  local.get $descriptor
                  call $drop-descriptor
                  local.get $preopen
                  call $drop-descriptor
                  i32.const 1
                else
                  local.get $stream
                  i64.const 4
                  i32.const 80
                  call $blocking-read
                  i32.const 80
                  i32.load8_u
                  if (result i32)
                    local.get $stream
                    call $drop-input-stream
                    local.get $descriptor
                    call $drop-descriptor
                    local.get $preopen
                    call $drop-descriptor
                    i32.const 1
                  else
                    i32.const 84
                    i32.load
                    i32.const 88
                    i32.load
                    call $write-stdout
                    local.get $stream
                    call $drop-input-stream
                    local.get $descriptor
                    call $drop-descriptor
                    local.get $preopen
                    call $drop-descriptor
                    i32.const 0
                  end
                end
              end
            end
          end
        end
      end
    end)
)
"#,
    )
    .expect("skip stream probe module を生成できる")
}

fn emit_component_cli_read_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.read" (func $read (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.blocking-read" (func $blocking-read (type 4)))
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
  (data (i32.const 144) "E")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $first-len i64)
    (local $remaining i64)
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
        i32.const 40
        call $read-via-stream
        i32.const 40
        i32.load8_u
        if (result i32)
          local.get $descriptor
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          i32.const 44
          i32.load
          local.set $stream
          local.get $stream
          i64.const 0
          i32.const 48
          call $read
          i32.const 48
          i32.load8_u
          if (result i32)
            local.get $stream
            call $drop-input-stream
            local.get $descriptor
            call $drop-descriptor
            local.get $preopen
            call $drop-descriptor
            i32.const 1
          else
            i32.const 56
            i32.load
            if (result i32)
              local.get $stream
              call $drop-input-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 1
            else
              local.get $stream
              i64.const 5
              i32.const 64
              call $read
              i32.const 64
              i32.load8_u
              if (result i32)
                local.get $stream
                call $drop-input-stream
                local.get $descriptor
                call $drop-descriptor
                local.get $preopen
                call $drop-descriptor
                i32.const 1
              else
                i32.const 72
                i32.load
                i64.extend_i32_u
                local.set $first-len
                local.get $first-len
                i64.const 5
                i64.gt_u
                if (result i32)
                  local.get $stream
                  call $drop-input-stream
                  local.get $descriptor
                  call $drop-descriptor
                  local.get $preopen
                  call $drop-descriptor
                  i32.const 1
                else
                  i32.const 68
                  i32.load
                  i32.const 72
                  i32.load
                  call $write-stdout
                  i64.const 5
                  local.get $first-len
                  i64.sub
                  local.set $remaining
                  local.get $stream
                  local.get $remaining
                  i32.const 80
                  call $blocking-read
                  i32.const 80
                  i32.load8_u
                  if (result i32)
                    local.get $stream
                    call $drop-input-stream
                    local.get $descriptor
                    call $drop-descriptor
                    local.get $preopen
                    call $drop-descriptor
                    i32.const 1
                  else
                    i32.const 88
                    i32.load
                    i64.extend_i32_u
                    local.get $remaining
                    i64.gt_u
                    if (result i32)
                      local.get $stream
                      call $drop-input-stream
                      local.get $descriptor
                      call $drop-descriptor
                      local.get $preopen
                      call $drop-descriptor
                      i32.const 1
                    else
                      i32.const 84
                      i32.load
                      i32.const 88
                      i32.load
                      call $write-stdout
                      local.get $stream
                      i64.const 1
                      i32.const 96
                      call $read
                      i32.const 96
                      i32.load8_u
                      if (result i32)
                        local.get $stream
                        call $drop-input-stream
                        local.get $descriptor
                        call $drop-descriptor
                        local.get $preopen
                        call $drop-descriptor
                        i32.const 1
                      else
                        i32.const 104
                        i32.load
                        if (result i32)
                          local.get $stream
                          call $drop-input-stream
                          local.get $descriptor
                          call $drop-descriptor
                          local.get $preopen
                          call $drop-descriptor
                          i32.const 1
                        else
                          i32.const 144
                          i32.const 1
                          call $write-stdout
                          local.get $stream
                          call $drop-input-stream
                          local.get $descriptor
                          call $drop-descriptor
                          local.get $preopen
                          call $drop-descriptor
                          i32.const 0
                        end
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end
    end)
)
"#,
    )
    .expect("read stream probe module を生成できる")
}

fn emit_component_cli_empty_read_stream_probe_module() -> Vec<u8> {
    emit_component_cli_empty_read_stream_probe_module_with_method("read", "Z")
}

fn emit_component_cli_empty_blocking_read_stream_probe_module() -> Vec<u8> {
    emit_component_cli_empty_read_stream_probe_module_with_method("blocking-read", "B")
}

fn emit_component_cli_empty_read_stream_probe_module_with_method(
    read_method: &str,
    marker: &str,
) -> Vec<u8> {
    let wat = r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.__READ_METHOD__" (func $__READ_METHOD__ (type 4)))
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
  (data (i32.const 144) "__READ_MARKER__")
  (data (i32.const 160) "C")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
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
        i32.const 40
        call $read-via-stream
        i32.const 40
        i32.load8_u
        if (result i32)
          local.get $descriptor
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          i32.const 44
          i32.load
          local.set $stream
          local.get $stream
          i64.const 1
          i32.const 48
          call $__READ_METHOD__
          i32.const 48
          i32.load8_u
          if (result i32)
            i32.const 52
            i32.load
            i32.const 1
            i32.eq
            if (result i32)
              i32.const 160
              i32.const 1
              call $write-stdout
              local.get $stream
              call $drop-input-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 0
            else
              local.get $stream
              call $drop-input-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 1
            end
          else
            i32.const 56
            i32.load
            if (result i32)
              local.get $stream
              call $drop-input-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 1
            else
              i32.const 144
              i32.const 1
              call $write-stdout
              local.get $stream
              call $drop-input-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 0
            end
          end
        end
      end
    end)
)
"#
    .replace("__READ_METHOD__", read_method)
    .replace("__READ_MARKER__", marker);
    wat::parse_str(wat).expect("empty read stream probe module を生成できる")
}
