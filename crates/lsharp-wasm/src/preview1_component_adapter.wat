(module
  (import "env" "memory" (memory 1))

  (import "wasi:cli/environment@0.2.3" "get-arguments" (func $get_arguments (param i32)))
  (import "wasi:cli/exit@0.2.3" "exit" (func $exit (param i32)))
  (import "wasi:cli/stdin@0.2.3" "get-stdin" (func $get_stdin (result i32)))
  (import "wasi:cli/stdout@0.2.3" "get-stdout" (func $get_stdout (result i32)))
  (import "wasi:cli/stderr@0.2.3" "get-stderr" (func $get_stderr (result i32)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get_directories (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $descriptor_open_at (param i32 i32 i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $descriptor_read_via_stream (param i32 i64 i32)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write-via-stream" (func $descriptor_write_via_stream (param i32 i64 i32)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.stat" (func $descriptor_stat (param i32 i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop_descriptor (param i32)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.blocking-read" (func $input_stream_blocking_read (param i32 i64 i32)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.blocking-write-and-flush" (func $output_stream_blocking_write_and_flush (param i32 i32 i32 i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]input-stream" (func $drop_input_stream (param i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]output-stream" (func $drop_output_stream (param i32)))
  (import "wasi:io/error@0.2.3" "[resource-drop]error" (func $drop_error (param i32)))

  (global $heap_ptr (mut i32) (i32.const 0))
  (global $stdin_handle (mut i32) (i32.const -1))
  (global $stdout_handle (mut i32) (i32.const -1))
  (global $stderr_handle (mut i32) (i32.const -1))
  (global $preopen_handle (mut i32) (i32.const -1))
  (global $open_desc_handle (mut i32) (i32.const -1))
  (global $open_offset (mut i64) (i64.const 0))

  (func $realloc_internal (param $old_ptr i32) (param $old_len i32) (param $align i32) (param $new_len i32) (result i32)
    (local $ptr i32)
    (local $end i32)
    (local $current i32)
    (local $copy_len i32)

    local.get $align
    i32.eqz
    if
      i32.const 1
      local.set $align
    end

    global.get $heap_ptr
    i32.eqz
    if
      memory.size
      i32.const 16
      i32.shl
      global.set $heap_ptr
    end

    global.get $heap_ptr
    local.get $align
    i32.const 1
    i32.sub
    i32.add
    i32.const 0
    local.get $align
    i32.sub
    i32.and
    local.set $ptr

    local.get $ptr
    local.get $new_len
    i32.add
    local.tee $end
    memory.size
    i32.const 16
    i32.shl
    local.tee $current
    i32.gt_u
    if
      local.get $end
      local.get $current
      i32.sub
      i32.const 65535
      i32.add
      i32.const 16
      i32.shr_u
      memory.grow
      i32.const -1
      i32.eq
      if
        unreachable
      end
    end

    local.get $old_ptr
    i32.const 0
    i32.ne
    if
      local.get $old_len
      local.get $new_len
      i32.lt_u
      if (result i32)
        local.get $old_len
      else
        local.get $new_len
      end
      local.set $copy_len

      local.get $copy_len
      i32.const 0
      i32.ne
      if
        local.get $ptr
        local.get $old_ptr
        local.get $copy_len
        memory.copy
      end
    end

    local.get $end
    global.set $heap_ptr
    local.get $ptr
  )

  (func $alloc (param $align i32) (param $size i32) (result i32)
    i32.const 0
    i32.const 0
    local.get $align
    local.get $size
    call $realloc_internal
  )

  (func (export "cabi_import_realloc") (param $old_ptr i32) (param $old_len i32) (param $align i32) (param $new_len i32) (result i32)
    local.get $old_ptr
    local.get $old_len
    local.get $align
    local.get $new_len
    call $realloc_internal
  )

  (func $get_stdin_cached (result i32)
    (local $handle i32)
    global.get $stdin_handle
    i32.const -1
    i32.ne
    if (result i32)
      global.get $stdin_handle
    else
      call $get_stdin
      local.tee $handle
      global.set $stdin_handle
      local.get $handle
    end
  )

  (func $get_stdout_cached (result i32)
    (local $handle i32)
    global.get $stdout_handle
    i32.const -1
    i32.ne
    if (result i32)
      global.get $stdout_handle
    else
      call $get_stdout
      local.tee $handle
      global.set $stdout_handle
      local.get $handle
    end
  )

  (func $get_stderr_cached (result i32)
    (local $handle i32)
    global.get $stderr_handle
    i32.const -1
    i32.ne
    if (result i32)
      global.get $stderr_handle
    else
      call $get_stderr
      local.tee $handle
      global.set $stderr_handle
      local.get $handle
    end
  )

  (func $get_preopen_cached (result i32)
    (local $ret i32)
    (local $list i32)
    (local $handle i32)
    global.get $preopen_handle
    i32.const -1
    i32.ne
    if (result i32)
      global.get $preopen_handle
    else
      i32.const 4
      i32.const 8
      call $alloc
      local.set $ret
      local.get $ret
      call $get_directories
      local.get $ret
      i32.load offset=4
      i32.eqz
      if (result i32)
        i32.const -1
      else
        local.get $ret
        i32.load
        local.set $list
        local.get $list
        i32.load
        local.tee $handle
        global.set $preopen_handle
        local.get $handle
      end
    end
  )

  (func $clear_open_file
    i32.const -1
    global.set $open_desc_handle
    i64.const 0
    global.set $open_offset
  )

  (func $drop_stream_error12 (param $ret i32)
    local.get $ret
    i32.load8_u offset=4
    i32.eqz
    if
      local.get $ret
      i32.load offset=8
      call $drop_error
    end
  )

  (func (export "proc_exit") (param $code i32)
    local.get $code
    i32.eqz
    if
      i32.const 0
      call $exit
    else
      i32.const 1
      call $exit
    end
  )

  (func (export "args_sizes_get") (param $argc_ptr i32) (param $argv_buf_size_ptr i32) (result i32)
    (local $ret i32)
    (local $list i32)
    (local $len i32)
    (local $i i32)
    (local $total i32)

    local.get $argc_ptr
    i32.const 0
    i32.store
    local.get $argv_buf_size_ptr
    i32.const 0
    i32.store

    i32.const 4
    i32.const 8
    call $alloc
    local.set $ret
    local.get $ret
    call $get_arguments

    local.get $ret
    i32.load
    local.set $list
    local.get $ret
    i32.load offset=4
    local.set $len

    local.get $argc_ptr
    local.get $len
    i32.store

    loop $count
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $total
        local.get $list
        local.get $i
        i32.const 8
        i32.mul
        i32.add
        i32.load offset=4
        i32.add
        i32.const 1
        i32.add
        local.set $total

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $count
      end
    end

    local.get $argv_buf_size_ptr
    local.get $total
    i32.store
    i32.const 0
  )

  (func (export "args_get") (param $argv_ptr i32) (param $argv_buf_ptr i32) (result i32)
    (local $ret i32)
    (local $list i32)
    (local $len i32)
    (local $i i32)
    (local $entry i32)
    (local $src i32)
    (local $src_len i32)
    (local $dst i32)

    local.get $argv_buf_ptr
    local.set $dst

    i32.const 4
    i32.const 8
    call $alloc
    local.set $ret
    local.get $ret
    call $get_arguments

    local.get $ret
    i32.load
    local.set $list
    local.get $ret
    i32.load offset=4
    local.set $len

    loop $fill
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $list
        local.get $i
        i32.const 8
        i32.mul
        i32.add
        local.tee $entry
        i32.load
        local.set $src
        local.get $entry
        i32.load offset=4
        local.set $src_len

        local.get $argv_ptr
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        local.get $dst
        i32.store

        local.get $dst
        local.get $src
        local.get $src_len
        memory.copy

        local.get $dst
        local.get $src_len
        i32.add
        i32.const 0
        i32.store8

        local.get $dst
        local.get $src_len
        i32.add
        i32.const 1
        i32.add
        local.set $dst

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $fill
      end
    end

    i32.const 0
  )

  (func (export "fd_close") (param $fd i32) (result i32)
    local.get $fd
    i32.const 3
    i32.le_u
    if
      i32.const 0
      return
    end

    local.get $fd
    i32.const 4
    i32.ne
    if
      i32.const 1
      return
    end

    global.get $open_desc_handle
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end

    global.get $open_desc_handle
    call $drop_descriptor
    call $clear_open_file
    i32.const 0
  )

  (func (export "fd_seek") (param $fd i32) (param $offset i64) (param $whence i32) (param $new_offset_ptr i32) (result i32)
    local.get $new_offset_ptr
    i64.const 0
    i64.store

    local.get $fd
    i32.const 4
    i32.ne
    if
      i32.const 1
      return
    end

    global.get $open_desc_handle
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end

    local.get $whence
    i32.const 0
    i32.eq
    if
      local.get $offset
      global.set $open_offset
    else
      local.get $whence
      i32.const 1
      i32.eq
      if
        global.get $open_offset
        local.get $offset
        i64.add
        global.set $open_offset
      else
        i32.const 1
        return
      end
    end

    local.get $new_offset_ptr
    global.get $open_offset
    i64.store
    i32.const 0
  )

  (func (export "fd_filestat_get") (param $fd i32) (param $buf_ptr i32) (result i32)
    (local $ret i32)

    local.get $buf_ptr
    i64.const 0
    i64.store
    local.get $buf_ptr
    i64.const 0
    i64.store offset=8
    local.get $buf_ptr
    i64.const 0
    i64.store offset=16
    local.get $buf_ptr
    i64.const 0
    i64.store offset=24
    local.get $buf_ptr
    i64.const 0
    i64.store offset=32
    local.get $buf_ptr
    i64.const 0
    i64.store offset=40
    local.get $buf_ptr
    i64.const 0
    i64.store offset=48
    local.get $buf_ptr
    i64.const 0
    i64.store offset=56

    local.get $fd
    i32.const 4
    i32.ne
    if
      i32.const 1
      return
    end

    global.get $open_desc_handle
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end

    i32.const 8
    i32.const 104
    call $alloc
    local.set $ret
    global.get $open_desc_handle
    local.get $ret
    call $descriptor_stat

    local.get $ret
    i32.load8_u
    i32.eqz
    if
      local.get $buf_ptr
      local.get $ret
      i64.load offset=24
      i64.store offset=32
      i32.const 0
      return
    end

    i32.const 1
  )

  (func (export "path_open")
    (param $dirfd i32)
    (param $dirflags i32)
    (param $path_ptr i32)
    (param $path_len i32)
    (param $oflags i32)
    (param $rights_base i64)
    (param $rights_inheriting i64)
    (param $fdflags i32)
    (param $fd_out_ptr i32)
    (result i32)
    (local $preopen i32)
    (local $path_flags i32)
    (local $open_flags i32)
    (local $descriptor_flags i32)
    (local $ret i32)
    (local $handle i32)

    local.get $fd_out_ptr
    i32.const 0
    i32.store

    call $get_preopen_cached
    local.tee $preopen
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end

    local.get $dirflags
    i32.const 1
    i32.and
    local.set $path_flags

    i32.const 0
    local.set $open_flags

    local.get $oflags
    i32.const 1
    i32.and
    i32.const 0
    i32.ne
    if
      local.get $open_flags
      i32.const 1
      i32.or
      local.set $open_flags
    end

    local.get $oflags
    i32.const 2
    i32.and
    i32.const 0
    i32.ne
    if
      local.get $open_flags
      i32.const 2
      i32.or
      local.set $open_flags
    end

    local.get $oflags
    i32.const 4
    i32.and
    i32.const 0
    i32.ne
    if
      local.get $open_flags
      i32.const 8
      i32.or
      local.set $open_flags
    end

    local.get $oflags
    i32.const 8
    i32.and
    i32.const 0
    i32.ne
    if
      local.get $open_flags
      i32.const 4
      i32.or
      local.set $open_flags
    end

    local.get $oflags
    i32.eqz
    if
      local.get $rights_base
      i64.const 64
      i64.eq
      if
        i32.const 2
        local.set $descriptor_flags
      else
        i32.const 1
        local.set $descriptor_flags
      end
    else
      i32.const 2
      local.set $descriptor_flags
    end

    i32.const 4
    i32.const 8
    call $alloc
    local.set $ret

    local.get $preopen
    local.get $path_flags
    local.get $path_ptr
    local.get $path_len
    local.get $open_flags
    local.get $descriptor_flags
    local.get $ret
    call $descriptor_open_at

    local.get $ret
    i32.load8_u
    i32.eqz
    if
      global.get $open_desc_handle
      i32.const -1
      i32.ne
      if
        local.get $ret
        i32.load offset=4
        call $drop_descriptor
        i32.const 1
        return
      end

      local.get $ret
      i32.load offset=4
      local.tee $handle
      global.set $open_desc_handle
      i64.const 0
      global.set $open_offset

      local.get $fd_out_ptr
      i32.const 4
      i32.store
      i32.const 0
      return
    end

    i32.const 1
  )

  (func (export "fd_read") (param $fd i32) (param $iovs i32) (param $iovs_len i32) (param $nread_ptr i32) (result i32)
    (local $buf_ptr i32)
    (local $buf_len i32)
    (local $ret i32)
    (local $stream i32)
    (local $bytes_ptr i32)
    (local $bytes_len i32)

    local.get $nread_ptr
    i32.const 0
    i32.store

    local.get $iovs_len
    i32.eqz
    if
      i32.const 0
      return
    end

    local.get $iovs
    i32.load
    local.set $buf_ptr
    local.get $iovs
    i32.load offset=4
    local.set $buf_len

    local.get $fd
    i32.const 0
    i32.eq
    if
      call $get_stdin_cached
      local.set $stream

      i32.const 4
      i32.const 12
      call $alloc
      local.set $ret
      local.get $stream
      local.get $buf_len
      i64.extend_i32_u
      local.get $ret
      call $input_stream_blocking_read

      local.get $ret
      i32.load8_u
      i32.eqz
      if
        local.get $ret
        i32.load offset=4
        local.set $bytes_ptr
        local.get $ret
        i32.load offset=8
        local.set $bytes_len

        local.get $buf_ptr
        local.get $bytes_ptr
        local.get $bytes_len
        memory.copy

        local.get $nread_ptr
        local.get $bytes_len
        i32.store
        i32.const 0
        return
      end

      local.get $ret
      call $drop_stream_error12
      i32.const 1
      return
    end

    local.get $fd
    i32.const 4
    i32.ne
    if
      i32.const 1
      return
    end

    global.get $open_desc_handle
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end

    i32.const 4
    i32.const 8
    call $alloc
    local.set $ret
    global.get $open_desc_handle
    global.get $open_offset
    local.get $ret
    call $descriptor_read_via_stream

    local.get $ret
    i32.load8_u
    i32.eqz
    if
      local.get $ret
      i32.load offset=4
      local.set $stream

      i32.const 4
      i32.const 12
      call $alloc
      local.set $ret
      local.get $stream
      local.get $buf_len
      i64.extend_i32_u
      local.get $ret
      call $input_stream_blocking_read

      local.get $ret
      i32.load8_u
      i32.eqz
      if
        local.get $ret
        i32.load offset=4
        local.set $bytes_ptr
        local.get $ret
        i32.load offset=8
        local.set $bytes_len

        local.get $buf_ptr
        local.get $bytes_ptr
        local.get $bytes_len
        memory.copy

        local.get $nread_ptr
        local.get $bytes_len
        i32.store

        global.get $open_offset
        local.get $bytes_len
        i64.extend_i32_u
        i64.add
        global.set $open_offset

        local.get $stream
        call $drop_input_stream
        i32.const 0
        return
      end

      local.get $ret
      call $drop_stream_error12
      local.get $stream
      call $drop_input_stream
      i32.const 1
      return
    end

    i32.const 1
  )

  (func $write_output_stream_all (param $stream i32) (param $buf_ptr i32) (param $buf_len i32) (param $nwritten_ptr i32) (result i32)
    (local $remaining i32)
    (local $chunk_len i32)
    (local $ret i32)

    local.get $buf_len
    local.set $remaining

    block $done
      loop $write_loop
        local.get $remaining
        i32.eqz
        br_if $done

        local.get $remaining
        local.set $chunk_len
        local.get $chunk_len
        i32.const 4096
        i32.gt_u
        if
          i32.const 4096
          local.set $chunk_len
        end

        i32.const 4
        i32.const 12
        call $alloc
        local.set $ret
        local.get $stream
        local.get $buf_ptr
        local.get $chunk_len
        local.get $ret
        call $output_stream_blocking_write_and_flush

        local.get $ret
        i32.load8_u
        i32.eqz
        if
          local.get $nwritten_ptr
          local.get $nwritten_ptr
          i32.load
          local.get $chunk_len
          i32.add
          i32.store

          local.get $buf_ptr
          local.get $chunk_len
          i32.add
          local.set $buf_ptr

          local.get $remaining
          local.get $chunk_len
          i32.sub
          local.set $remaining
          br $write_loop
        end

        local.get $ret
        call $drop_stream_error12
        i32.const 1
        return
      end
    end

    i32.const 0
  )

  (func (export "fd_write") (param $fd i32) (param $iovs i32) (param $iovs_len i32) (param $nwritten_ptr i32) (result i32)
    (local $buf_ptr i32)
    (local $buf_len i32)
    (local $stream i32)
    (local $ret i32)

    local.get $nwritten_ptr
    i32.const 0
    i32.store

    local.get $iovs_len
    i32.eqz
    if
      i32.const 0
      return
    end

    local.get $iovs
    i32.load
    local.set $buf_ptr
    local.get $iovs
    i32.load offset=4
    local.set $buf_len

    local.get $fd
    i32.const 1
    i32.eq
    if
      call $get_stdout_cached
      local.set $stream
      local.get $stream
      local.get $buf_ptr
      local.get $buf_len
      local.get $nwritten_ptr
      call $write_output_stream_all
      return
    end

    local.get $fd
    i32.const 2
    i32.eq
    if
      call $get_stderr_cached
      local.set $stream
      local.get $stream
      local.get $buf_ptr
      local.get $buf_len
      local.get $nwritten_ptr
      call $write_output_stream_all
      return
    end

    local.get $fd
    i32.const 4
    i32.ne
    if
      i32.const 1
      return
    end

    global.get $open_desc_handle
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end

    i32.const 4
    i32.const 8
    call $alloc
    local.set $ret
    global.get $open_desc_handle
    global.get $open_offset
    local.get $ret
    call $descriptor_write_via_stream

    local.get $ret
    i32.load8_u
    i32.eqz
    if
      local.get $ret
      i32.load offset=4
      local.set $stream

      local.get $stream
      local.get $buf_ptr
      local.get $buf_len
      local.get $nwritten_ptr
      call $write_output_stream_all
      local.set $ret

      global.get $open_offset
      local.get $nwritten_ptr
      i32.load
      i64.extend_i32_u
      i64.add
      global.set $open_offset

      local.get $stream
      call $drop_output_stream
      local.get $ret
      return
    end

    i32.const 1
  )
)
