fn emit_component_cli_nonblocking_input_skip_failure_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32 i32)))
  (type (func (param i32) (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.skip" (func $skip (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.subscribe" (func $subscribe (type 6)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.block" (func $block (type 1)))
  (import "wasi:filesystem/types@0.2.3" "filesystem-error-code" (func $filesystem-error-code (type 5)))
  (import "wasi:io/poll@0.2.3" "[resource-drop]pollable" (func $drop-pollable (param i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]input-stream" (func $drop-input-stream (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:io/error@0.2.3" "[resource-drop]error" (func $drop-error (param i32)))
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
  (data (i32.const 144) "S")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $pollable i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 1
    i32.ne
    if
      i32.const 1
      return
    end
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
    i64.const -1
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
    local.get $stream
    i64.const 1
    i32.const 48
    call $skip
    i32.const 48
    i32.load8_u
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
    i32.const 56
    i32.load8_u
    i32.const 0
    i32.ne
    if
      i32.const 60
      i32.load
      call $drop-error
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
    i32.const 60
    i32.load
    i32.const 64
    call $filesystem-error-code
    i32.const 64
    i32.load8_u
    i32.const 1
    i32.ne
    if
      i32.const 60
      i32.load
      call $drop-error
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
    i32.const 65
    i32.load8_u
    i32.const 12
    i32.ne
    if
      i32.const 60
      i32.load
      call $drop-error
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
    call $write-stdout
    i32.const 60
    i32.load
    call $drop-error
    local.get $pollable
    call $drop-pollable
    local.get $stream
    call $drop-input-stream
    local.get $descriptor
    call $drop-descriptor
    local.get $preopen
    call $drop-descriptor
    i32.const 0)
)
"#,
    )
    .expect("non-blocking input skip failure probe module を生成できる")
}

fn emit_component_cli_direct_write_zeroes_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 2)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write-via-stream" (func $write-via-stream (type 3)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.check-write" (func $check-write (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.write-zeroes" (func $write-zeroes (type 3)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.blocking-flush" (func $blocking-flush (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]output-stream" (func $drop-output-stream (param i32)))
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
  (data (i32.const 128) "direct-zeroes.bin")
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
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
      i32.const 17
      i32.const 9
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
        i64.const 0
        i32.const 40
        call $write-via-stream
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
          i32.const 48
          call $check-write
          i32.const 48
          i32.load8_u
          if (result i32)
            local.get $stream
            call $drop-output-stream
            local.get $descriptor
            call $drop-descriptor
            local.get $preopen
            call $drop-descriptor
            i32.const 1
          else
            i32.const 56
            i64.load
            i64.const 4
            i64.lt_u
            if (result i32)
              local.get $stream
              call $drop-output-stream
              local.get $descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 1
            else
              local.get $stream
              i64.const 4
              i32.const 64
              call $write-zeroes
              i32.const 64
              i32.load8_u
              if (result i32)
                local.get $stream
                call $drop-output-stream
                local.get $descriptor
                call $drop-descriptor
                local.get $preopen
                call $drop-descriptor
                i32.const 1
              else
                local.get $stream
                i32.const 72
                call $blocking-flush
                i32.const 72
                i32.load8_u
                if (result i32)
                  local.get $stream
                  call $drop-output-stream
                  local.get $descriptor
                  call $drop-descriptor
                  local.get $preopen
                  call $drop-descriptor
                  i32.const 1
                else
                  local.get $stream
                  call $drop-output-stream
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
    end)
)
"#,
    )
    .expect("direct write-zeroes stream probe module を生成できる")
}

fn emit_component_cli_splice_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32 i32 i64 i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 0)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 2)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write-via-stream" (func $write-via-stream (type 3)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.splice" (func $splice (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.blocking-splice" (func $blocking-splice (type 4)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]input-stream" (func $drop-input-stream (param i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]output-stream" (func $drop-output-stream (param i32)))
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
  (data (i32.const 256) "spliced.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    (local $preopen i32)
    (local $input-descriptor i32)
    (local $output-descriptor i32)
    (local $input-stream i32)
    (local $output-stream i32)
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
        local.set $input-descriptor
        local.get $preopen
        i32.const 0
        i32.const 256
        i32.const 11
        i32.const 9
        i32.const 2
        i32.const 40
        call $open-at
        i32.const 40
        i32.load8_u
        if (result i32)
          local.get $input-descriptor
          call $drop-descriptor
          local.get $preopen
          call $drop-descriptor
          i32.const 1
        else
          i32.const 44
          i32.load
          local.set $output-descriptor
          local.get $input-descriptor
          i64.const 0
          i32.const 48
          call $read-via-stream
          i32.const 48
          i32.load8_u
          if (result i32)
            local.get $output-descriptor
            call $drop-descriptor
            local.get $input-descriptor
            call $drop-descriptor
            local.get $preopen
            call $drop-descriptor
            i32.const 1
          else
            i32.const 52
            i32.load
            local.set $input-stream
            local.get $output-descriptor
            i64.const 0
            i32.const 56
            call $write-via-stream
            i32.const 56
            i32.load8_u
            if (result i32)
              local.get $input-stream
              call $drop-input-stream
              local.get $output-descriptor
              call $drop-descriptor
              local.get $input-descriptor
              call $drop-descriptor
              local.get $preopen
              call $drop-descriptor
              i32.const 1
            else
              i32.const 60
              i32.load
              local.set $output-stream
              local.get $output-stream
              local.get $input-stream
              i64.const 5
              i32.const 64
              call $splice
              i32.const 64
              i32.load8_u
              if (result i32)
                local.get $output-stream
                call $drop-output-stream
                local.get $input-stream
                call $drop-input-stream
                local.get $output-descriptor
                call $drop-descriptor
                local.get $input-descriptor
                call $drop-descriptor
                local.get $preopen
                call $drop-descriptor
                i32.const 1
              else
                local.get $output-stream
                local.get $input-stream
                i64.const 5
                i32.const 80
                call $blocking-splice
                i32.const 80
                i32.load8_u
                if (result i32)
                  local.get $output-stream
                  call $drop-output-stream
                  local.get $input-stream
                  call $drop-input-stream
                  local.get $output-descriptor
                  call $drop-descriptor
                  local.get $input-descriptor
                  call $drop-descriptor
                  local.get $preopen
                  call $drop-descriptor
                  i32.const 1
                else
                  local.get $output-stream
                  call $drop-output-stream
                  local.get $input-stream
                  call $drop-input-stream
                  local.get $output-descriptor
                  call $drop-descriptor
                  local.get $input-descriptor
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
    end)
)
"#,
    )
    .expect("splice stream probe module を生成できる")
}
