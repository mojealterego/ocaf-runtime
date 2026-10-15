;; Minimal REAL WASI Preview 1 module — not simulated output from PID1.
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 64) "OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE\0a")
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 64))
    (i32.store (i32.const 4) (i32.const 42))
    (if (i32.ne
          (call $fd_write (i32.const 1) (i32.const 0)
                         (i32.const 1) (i32.const 16))
          (i32.const 0))
      (then unreachable))
  )
)
