; OCAF stage 334: bounded caveat monotonicity in integer verb-mask model.
; UNSAT: a child cannot gain Execute=4 when parent mask lacks Execute.
(set-logic QF_BV)
(declare-const parent (_ BitVec 3))
(declare-const child (_ BitVec 3))
(assert (= parent #b001))
; Child permissions are a bitwise subset of parent.
(assert (= (bvand child parent) child))
; Attack tries to gain Execute (bit 2).
(assert (not (= (bvand child #b100) #b000)))
(check-sat)
