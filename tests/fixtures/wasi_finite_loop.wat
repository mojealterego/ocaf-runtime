(module
  (func (export "_start")
    (local $counter i32)
    (local.set $counter (i32.const 8))
    (block $done
      (loop $again
        (local.set $counter (i32.sub (local.get $counter) (i32.const 1)))
        (br_if $done (i32.eqz (local.get $counter)))
        (br $again)
      )
    )
  )
)
